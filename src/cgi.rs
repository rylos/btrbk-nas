//! The CGI side of a request: DSM's web server runs the program once per
//! request, with the request in the environment and the body on stdin.

use std::collections::HashMap;
use std::env;
use std::io::{self, Read, Write};

use serde::Serialize;

/// Largest body accepted: settings are a few hundred bytes.
const MAX_BODY: usize = 64 * 1024;

pub struct Request {
    pub method: String,
    /// Path below the CGI, without the leading slash: "overview", "browse"...
    pub route: String,
    pub query: HashMap<String, String>,
    pub content_type: String,
    pub body: Vec<u8>,
}

impl Request {
    pub fn from_env() -> io::Result<Self> {
        let method = env::var("REQUEST_METHOD").unwrap_or_else(|_| "GET".into());
        let route = env::var("PATH_INFO").unwrap_or_default().trim_matches('/').to_string();
        let query = parse_query(&env::var("QUERY_STRING").unwrap_or_default());
        let content_type = env::var("CONTENT_TYPE").unwrap_or_default();
        let length: usize = env::var("CONTENT_LENGTH").ok().and_then(|l| l.parse().ok()).unwrap_or(0);
        if length > MAX_BODY {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "request body too large"));
        }
        let mut body = vec![0; length];
        io::stdin().read_exact(&mut body)?;
        Ok(Self { method, route, query, content_type, body })
    }

    pub fn param(&self, name: &str) -> Option<&str> {
        self.query.get(name).map(String::as_str)
    }
}

pub fn parse_query(query: &str) -> HashMap<String, String> {
    query
        .split('&')
        .filter(|pair| !pair.is_empty())
        .map(|pair| {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            (percent_decode(key), percent_decode(value))
        })
        .collect()
}

pub fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => out.push(b' '),
            b'%' if i + 2 < bytes.len() => match (hex(bytes[i + 1]), hex(bytes[i + 2])) {
                (Some(high), Some(low)) => {
                    out.push(high << 4 | low);
                    i += 2;
                }
                _ => out.push(b'%'),
            },
            byte => out.push(byte),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex(digit: u8) -> Option<u8> {
    (digit as char).to_digit(16).map(|d| d as u8)
}

/// RFC 5987 encoding for the filename* parameter of Content-Disposition.
pub fn percent_encode(text: &str) -> String {
    text.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

/// An error answered as JSON with an HTTP status.
#[derive(Debug)]
pub struct ApiError {
    pub status: u16,
    pub message: String,
}

impl ApiError {
    pub fn new(status: u16, message: impl Into<String>) -> Self {
        Self { status, message: message.into() }
    }
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new(400, message)
    }
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(404, message)
    }
}

impl From<io::Error> for ApiError {
    fn from(e: io::Error) -> Self {
        let status = match e.kind() {
            io::ErrorKind::NotFound => 404,
            io::ErrorKind::PermissionDenied => 403,
            io::ErrorKind::InvalidInput => 400,
            _ => 500,
        };
        Self::new(status, e.to_string())
    }
}

fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        409 => "Conflict",
        502 => "Bad Gateway",
        _ => "Internal Server Error",
    }
}

fn head(out: &mut impl Write, status: u16, headers: &[(&str, &str)]) -> io::Result<()> {
    write!(out, "Status: {status} {}\r\n", reason(status))?;
    for (name, value) in headers {
        write!(out, "{name}: {value}\r\n")?;
    }
    // API answers are per-user and change constantly
    write!(out, "Cache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\n\r\n")
}

pub fn json(out: &mut impl Write, status: u16, value: &impl Serialize) -> io::Result<()> {
    let body = serde_json::to_vec(value).map_err(io::Error::other)?;
    let length = body.len().to_string();
    head(
        out,
        status,
        &[("Content-Type", "application/json; charset=utf-8"), ("Content-Length", &length)],
    )?;
    out.write_all(&body)
}

pub fn error(out: &mut impl Write, err: &ApiError) -> io::Result<()> {
    json(out, err.status, &serde_json::json!({ "error": err.message }))
}

/// Starts a download; the caller writes the body.
pub fn attachment(out: &mut impl Write, content_type: &str, filename: &str, length: Option<u64>) -> io::Result<()> {
    // ASCII fallback for old clients, the exact name in filename*
    let fallback: String = filename
        .chars()
        .map(|c| if c.is_ascii_graphic() && c != '"' && c != '\\' { c } else { '_' })
        .collect();
    let disposition = format!("attachment; filename=\"{fallback}\"; filename*=UTF-8''{}", percent_encode(filename));
    let length = length.map(|l| l.to_string());
    let mut headers = vec![("Content-Type", content_type), ("Content-Disposition", disposition.as_str())];
    if let Some(length) = &length {
        headers.push(("Content-Length", length));
    }
    head(out, 200, &headers)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_queries() {
        let q = parse_query("id=0&snapshot=%40home.20261026T0847&path=alice%2Fa+b%2F%C3%A8&bad=%zz&x");
        assert_eq!(q["snapshot"], "@home.20261026T0847");
        assert_eq!(q["path"], "alice/a b/è");
        assert_eq!(q["bad"], "%zz");
        assert_eq!(q["x"], "");
        assert_eq!(percent_decode("100%"), "100%");
        assert_eq!(percent_decode("%4"), "%4");
    }

    #[test]
    fn encodes_filenames() {
        assert_eq!(percent_encode("café è.txt"), "caf%C3%A9%20%C3%A8.txt");
    }
}
