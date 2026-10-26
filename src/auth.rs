//! Who is calling. DSM checks its own session cookie: authenticate.cgi prints
//! the user name of a valid session and nothing otherwise. It reads the
//! request from the environment, which a CGI child inherits as it is.

use std::fs;
use std::process::{Command, Stdio};

use crate::cgi::ApiError;

const AUTHENTICATE: &str = "/usr/syno/synoman/webman/modules/authenticate.cgi";
const ADMIN_GROUP: &str = "administrators";

/// The DSM user behind the request, refused unless an administrator: the
/// package reads every file of every backup.
pub fn require_admin() -> Result<String, ApiError> {
    // debug builds only, for the local development server
    #[cfg(debug_assertions)]
    if let Ok(user) = std::env::var("BTRBK_NAS_DEV_USER") {
        return Ok(user);
    }
    let output = Command::new(AUTHENTICATE)
        .env_clear()
        .envs(std::env::vars_os().filter(|(name, _)| !is_unsafe_env(&name.to_string_lossy())))
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .map_err(|e| ApiError::new(500, format!("cannot run {AUTHENTICATE}: {e}")))?;
    let user = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if user.is_empty() {
        return Err(ApiError::new(401, "not logged in to DSM"));
    }
    let groups = fs::read_to_string("/etc/group").unwrap_or_default();
    if !is_member(&groups, ADMIN_GROUP, &user) {
        return Err(ApiError::new(403, format!("{user} is not an administrator")));
    }
    Ok(user)
}

/// Variables that change how a dynamically linked program loads or behaves.
/// This process may hold capabilities, authenticate.cgi does not: a local
/// user must not get it to print an administrator's name through LD_PRELOAD.
/// The list is glibc's own for setuid programs (unsecvars.h).
fn is_unsafe_env(name: &str) -> bool {
    name.starts_with("LD_")
        || matches!(
            name,
            "GCONV_PATH"
                | "GETCONF_DIR"
                | "HOSTALIASES"
                | "LOCALDOMAIN"
                | "LOCPATH"
                | "MALLOC_TRACE"
                | "NIS_PATH"
                | "NLSPATH"
                | "RESOLV_HOST_CONF"
                | "RES_OPTIONS"
                | "TMPDIR"
                | "TZDIR"
        )
}

/// Whether `user` is listed in `group` of an /etc/group file. DSM user names
/// are case-insensitive.
pub fn is_member(groups: &str, group: &str, user: &str) -> bool {
    groups.lines().any(|line| {
        let mut fields = line.split(':');
        fields.next() == Some(group)
            && fields.nth(2).is_some_and(|members| members.split(',').any(|m| m.eq_ignore_ascii_case(user)))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drops_loader_variables() {
        assert!(is_unsafe_env("LD_PRELOAD"));
        assert!(is_unsafe_env("LD_LIBRARY_PATH"));
        assert!(!is_unsafe_env("HTTP_COOKIE"));
        assert!(!is_unsafe_env("REMOTE_ADDR"));
    }

    #[test]
    fn finds_group_members() {
        let groups = "users:x:100:\nadministrators:x:101:admin,Alice\n";
        assert!(is_member(groups, "administrators", "alice"));
        assert!(is_member(groups, "administrators", "admin"));
        assert!(!is_member(groups, "administrators", "bob"));
        assert!(!is_member(groups, "users", "alice"));
    }
}
