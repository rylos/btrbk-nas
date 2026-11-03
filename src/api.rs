//! The JSON API behind the interface.

use std::collections::BTreeMap;
use std::io::{self, BufWriter, Write};
use std::path::Path;

use serde_json::json;

use crate::browse;
use crate::cgi::{self, ApiError, Request};
use crate::config::{self, Config, Target};
use crate::lang::t;
use crate::monitor;
use crate::report;
use crate::snapshot;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

fn overview(config: &Config, user: &str) -> serde_json::Value {
    let now = report::now();
    json!({
        "version": VERSION,
        "user": user,
        "capabilities": crate::caps::granted(),
        "credits": config.show_credits,
        "synofile": synofile(),
        "theme": themes().remove(&user.to_lowercase()),
        "asset": option_env!("BTRBK_NAS_ASSET").unwrap_or("dev"),
        "now": now,
        "monitor": monitor_status(config),
        "targets": report::assess_all(config, now),
    })
}

/// Whether folders open in SynoFile instead of File Station: from 0.8.11 it
/// takes the folder to open when launched.
fn synofile() -> bool {
    let info = std::fs::read_to_string("/var/packages/synofile/INFO").unwrap_or_default();
    info.lines().find_map(|l| l.strip_prefix("version=")).is_some_and(|v| at_least(v.trim_matches('"'), &[0, 8, 11]))
}

/// "0.8.11-0001" >= [0, 8, 11]; the build number does not count.
fn at_least(version: &str, min: &[u64]) -> bool {
    let parts: Vec<u64> = version.split('-').next().unwrap_or("").split('.').map_while(|p| p.parse().ok()).collect();
    parts.as_slice() >= min
}

/// Whether the periodic check runs, and when it last looked.
fn monitor_status(config: &Config) -> serde_json::Value {
    json!({
        "enabled": config.notifications.enabled,
        "running": monitor::running(),
        "last_check": monitor::State::load(&monitor::State::path()).last_check,
        "interval_minutes": config.notifications.interval_minutes,
    })
}

fn target<'a>(config: &'a Config, request: &Request) -> Result<&'a Target, ApiError> {
    request
        .param("id")
        .and_then(|id| id.parse::<usize>().ok())
        .and_then(|id| config.targets.get(id))
        .ok_or_else(|| ApiError::not_found(t!("cartella di backup inesistente", "no such target")))
}

fn target_detail(config: &Config, request: &Request) -> Result<serde_json::Value, ApiError> {
    let target = target(config, request)?;
    let thresholds = config.thresholds(target);
    let now = report::now();
    let (groups, error, source) = match snapshot::scan(&target.path) {
        Ok(scan) => {
            let groups: Vec<_> = scan
                .groups
                .iter()
                .map(|g| json!({ "summary": report::summarise(g, now, thresholds), "snapshots": g.snapshots }))
                .collect();
            let source = report::source_info(&scan.source);
            (groups, scan.metadata_error.map(|e| t!("metadati btrfs non leggibili: {}", "cannot read btrfs metadata: {}", e)), source)
        }
        // the page still opens: it says what is wrong with the directory
        Err(e) => (Vec::new(), Some(format!("{}: {e}", target.path.display())), None),
    };
    Ok(json!({
        "label": target.display_name(),
        "path": target.path,
        "groups": groups,
        "error": error,
        "source": source,
    }))
}

fn snapshot_item(config: &Config, request: &Request) -> Result<browse::Item, ApiError> {
    let target = target(config, request)?;
    let name = request.param("snapshot").ok_or_else(|| ApiError::bad_request(t!("snapshot mancante", "snapshot missing")))?;
    browse::resolve(&target.path, name, request.param("path").unwrap_or(""))
}

fn download(config: &Config, request: &Request, out: &mut impl Write) -> Result<(), ApiError> {
    let item = snapshot_item(config, request)?;
    if item.meta.is_dir() {
        cgi::attachment(out, "application/x-tar", &format!("{}.tar", item.name), None)?;
        browse::write_tar(item.handle(), &item.name, out)?;
    } else if item.meta.is_file() {
        let file = item.open_file()?;
        let length = item.meta.len();
        cgi::attachment(out, "application/octet-stream", &item.name, Some(length))?;
        io::copy(&mut io::Read::take(file, length), out)?;
    } else {
        return Err(ApiError::bad_request(t!(
            "si possono scaricare solo file e cartelle",
            "only files and directories can be downloaded"
        )));
    }
    Ok(())
}

fn save_settings(request: &Request, path: &Path) -> Result<Config, ApiError> {
    let config: Config =
        serde_json::from_slice(&request.body).map_err(|e| ApiError::bad_request(t!("impostazioni non valide: {}", "invalid settings: {}", e)))?;
    config.validate().map_err(ApiError::bad_request)?;
    for target in &config.targets {
        if !target.path.is_dir() {
            return Err(ApiError::bad_request(t!("{}: cartella inesistente", "{}: no such directory", target.path.display())));
        }
    }
    config.save(path)?;
    Ok(config)
}

/// Deletes one snapshot, if it may go: the checks run again on the NAS,
/// whatever the interface showed.
fn delete_snapshot(config: &Config, request: &Request, user: &str) -> Result<serde_json::Value, ApiError> {
    #[derive(serde::Deserialize)]
    struct Body {
        id: usize,
        // the folder the page meant: the id is a position in the settings,
        // which may have changed since the page was drawn
        path: Option<std::path::PathBuf>,
        snapshot: String,
    }
    let body: Body =
        serde_json::from_slice(&request.body).map_err(|e| ApiError::bad_request(t!("richiesta non valida: {}", "invalid request: {}", e)))?;
    let target =
        config.targets.get(body.id).ok_or_else(|| ApiError::not_found(t!("cartella di backup inesistente", "no such target")))?;
    if body.path.as_ref().is_some_and(|p| *p != target.path) {
        return Err(ApiError::new(
            409,
            t!("le cartelle di backup sono cambiate: ricarica la pagina", "the backup folders have changed: reload the page"),
        ));
    }
    let lock = config::var_dir().join("delete.lock");
    match snapshot::delete(&target.path, &body.snapshot, &lock) {
        Ok(()) => {
            monitor::log(&format!("{user} deleted {}/{}", target.path.display(), body.snapshot));
            Ok(json!({ "deleted": body.snapshot }))
        }
        Err(e) => Err(ApiError::new(e.status(), e.to_string())),
    }
}

/// Sends the test notification with the channels as set in the form, saved
/// or not, so they can be tried before saving.
fn notify_test(request: &Request, user: &str) -> Result<serde_json::Value, ApiError> {
    let notifications: config::Notifications =
        serde_json::from_slice(&request.body).map_err(|e| ApiError::bad_request(t!("impostazioni non valide: {}", "invalid settings: {}", e)))?;
    let check = Config { notifications: notifications.clone(), ..Config::default() };
    check.validate().map_err(ApiError::bad_request)?;
    if !notifications.desktop && !notifications.webhook.enabled {
        return Err(ApiError::bad_request(t!("nessun canale di notifica attivo", "no notification channel is enabled")));
    }
    monitor::send(&monitor::Event::test(user), &notifications)
        .map_err(|e| ApiError::new(502, t!("notifica non inviata: {}", "cannot notify: {}", e)))?;
    Ok(json!({ "sent": true }))
}

/// The theme each admin picked, kept on the NAS: a browser can forget its
/// own storage.
fn themes() -> BTreeMap<String, String> {
    std::fs::read(config::var_dir().join("themes.json")).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
}

fn save_theme(request: &Request, user: &str) -> Result<serde_json::Value, ApiError> {
    #[derive(serde::Deserialize)]
    struct Body {
        theme: String,
    }
    let body: Body =
        serde_json::from_slice(&request.body).map_err(|e| ApiError::bad_request(t!("richiesta non valida: {}", "invalid request: {}", e)))?;
    if body.theme != "light" && body.theme != "dark" {
        return Err(ApiError::bad_request(t!("tema non valido", "invalid theme")));
    }
    // two windows saving at once: one read-modify-write at a time, or a
    // theme saved by one is lost to the other
    let _lock = config::lock(&config::var_dir().join("themes.lock"))?;
    let mut all = themes();
    all.insert(user.to_lowercase(), body.theme);
    config::write_atomic(&config::var_dir().join("themes.json"), &serde_json::to_vec(&all).map_err(io::Error::other)?)?;
    Ok(json!({ "ok": true }))
}

/// Targets found on the volumes, with the label they would get.
fn discover() -> serde_json::Value {
    let found: Vec<_> = snapshot::discover(Path::new("/"))
        .into_iter()
        .map(|path| json!({ "label": config::default_label(&path), "path": path }))
        .collect();
    json!({ "targets": found })
}

fn route(request: &Request, out: &mut impl Write) -> Result<(), ApiError> {
    if request.route == "health" {
        // no login: says the CGI runs, nothing else
        return Ok(cgi::json(out, 200, &json!({ "ok": true }))?);
    }
    let user = crate::auth::require_admin()?;
    crate::caps::limit(capabilities(&request.method, &request.route))?;
    let config_path = Config::path();
    let mut config = match Config::load(&config_path) {
        Ok(config) => config,
        // saving replaces the file whole: broken settings must not stop the
        // way out of them
        Err(e) if request.method == "POST" && request.route == "settings" => {
            monitor::log(&format!("settings unreadable, saved over: {e}"));
            Config::default()
        }
        Err(e) => return Err(e.into()),
    };

    let writes = request.method == "POST";
    if writes && !request.content_type.starts_with("application/json") {
        // a cross-site form can only post form encodings or text/plain
        return Err(ApiError::bad_request(t!("attesa una richiesta application/json", "expected application/json")));
    }

    match (request.method.as_str(), request.route.as_str()) {
        ("GET", "overview") => {
            // first run: offer what is already on the volumes
            if config.targets.is_empty() && !config_path.exists() {
                config.targets = snapshot::discover(Path::new("/")).into_iter().map(Target::new).collect();
                // webhook texts in the language of whoever opens the app first
                config.notifications.language = crate::lang::code().into();
                config.save(&config_path)?;
            }
            cgi::json(out, 200, &overview(&config, &user))?;
        }
        ("GET", "target") => cgi::json(out, 200, &target_detail(&config, request)?)?,
        ("GET", "browse") => cgi::json(out, 200, &browse::list(snapshot_item(&config, request)?.handle())?)?,
        ("GET", "download") => download(&config, request, out)?,
        ("GET", "settings") => cgi::json(out, 200, &config)?,
        ("POST", "settings") => cgi::json(out, 200, &save_settings(request, &config_path)?)?,
        ("POST", "discover") => cgi::json(out, 200, &discover())?,
        ("POST", "notify-test") => cgi::json(out, 200, &notify_test(request, &user)?)?,
        ("POST", "delete") => cgi::json(out, 200, &delete_snapshot(&config, request, &user)?)?,
        ("POST", "theme") => cgi::json(out, 200, &save_theme(request, &user)?)?,
        ("GET", "monitor") => cgi::json(out, 200, &monitor_status(&config))?,
        (_, "overview" | "target" | "browse" | "download" | "settings" | "discover" | "notify-test" | "monitor" | "delete" | "theme") => {
            return Err(ApiError::new(405, t!("metodo non consentito", "method not allowed")));
        }
        _ => return Err(ApiError::not_found(t!("API inesistente", "no such API"))),
    }
    Ok(())
}

/// What a route needs of the capabilities; the rest is dropped before it
/// runs.
fn capabilities(method: &str, route: &str) -> u64 {
    use crate::caps::{ALL, DAC_READ_SEARCH, READ};
    match (method, route) {
        ("GET", "overview" | "target" | "browse" | "download") => READ,
        // stat and readdir only: is it a directory, does it hold snapshots
        ("POST", "settings" | "discover") => DAC_READ_SEARCH,
        ("POST", "delete") => ALL,
        _ => 0,
    }
}

/// The reply, noting whether any of it has been written: from then on an
/// error can no longer be answered as an error.
struct Reply<W: Write> {
    out: W,
    started: bool,
}

impl<W: Write> Write for Reply<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.started |= !buf.is_empty();
        self.out.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.out.flush()
    }
}

/// Serves one CGI request.
pub fn serve() -> io::Result<()> {
    // nothing runs with the capabilities before the login check
    crate::caps::suspend()?;
    let stdout = io::stdout();
    let mut reply = Reply { out: BufWriter::with_capacity(256 * 1024, stdout.lock()), started: false };
    let result = Request::from_env().and_then(|request| {
        crate::lang::set(request.param("lang"));
        route(&request, &mut reply)
    });
    match result {
        Ok(()) => {}
        // a download already under way: an error reply now would end up
        // inside the file. Cut it short instead, so the browser or tar sees
        // a truncated download, never a complete-looking one
        Err(err) if reply.started => monitor::log(&format!("reply cut short: {}", err.message)),
        Err(err) => cgi::error(&mut reply, &err)?,
    }
    reply.flush()
}


#[cfg(test)]
mod tests {
    use super::at_least;

    #[test]
    fn compares_package_versions() {
        assert!(at_least("0.8.11-0001", &[0, 8, 11]));
        assert!(at_least("0.9.0-0001", &[0, 8, 11]));
        assert!(at_least("1.0", &[0, 8, 11]));
        assert!(!at_least("0.8.10-0003", &[0, 8, 11]));
        assert!(!at_least("", &[0, 8, 11]));
    }
}
