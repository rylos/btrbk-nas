//! The JSON API behind the interface.

use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};

use serde_json::json;

use crate::browse;
use crate::cgi::{self, ApiError, Request};
use crate::config::{self, Config, Target};
use crate::monitor;
use crate::report;
use crate::snapshot;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

fn overview(config: &Config, user: &str) -> serde_json::Value {
    let now = report::now();
    json!({
        "version": VERSION,
        "user": user,
        "capabilities": crate::caps::effective(),
        "now": now,
        "monitor": monitor_status(config),
        "targets": report::assess_all(config, now),
    })
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
        .ok_or_else(|| ApiError::not_found("no such target"))
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
            (groups, scan.metadata_error.map(|e| format!("cannot read btrfs metadata: {e}")), source)
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

fn snapshot_path(config: &Config, request: &Request) -> Result<PathBuf, ApiError> {
    let target = target(config, request)?;
    let name = request.param("snapshot").ok_or_else(|| ApiError::bad_request("snapshot missing"))?;
    browse::resolve(&target.path, name, request.param("path").unwrap_or(""))
}

fn download(config: &Config, request: &Request, out: &mut impl Write) -> Result<(), ApiError> {
    let path = snapshot_path(config, request)?;
    let meta = std::fs::symlink_metadata(&path)?;
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "download".into());
    if meta.is_dir() {
        cgi::attachment(out, "application/x-tar", &format!("{name}.tar"), None)?;
        browse::write_tar(&path, out)?;
    } else if meta.is_file() {
        let mut file = std::fs::File::open(&path)?;
        cgi::attachment(out, "application/octet-stream", &name, Some(meta.len()))?;
        io::copy(&mut file, out)?;
    } else {
        return Err(ApiError::bad_request("only files and directories can be downloaded"));
    }
    Ok(())
}

fn save_settings(request: &Request, path: &Path) -> Result<Config, ApiError> {
    let config: Config =
        serde_json::from_slice(&request.body).map_err(|e| ApiError::bad_request(format!("invalid settings: {e}")))?;
    config.validate().map_err(ApiError::bad_request)?;
    for target in &config.targets {
        if !target.path.is_dir() {
            return Err(ApiError::bad_request(format!("{}: no such directory", target.path.display())));
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
        snapshot: String,
    }
    let body: Body =
        serde_json::from_slice(&request.body).map_err(|e| ApiError::bad_request(format!("invalid request: {e}")))?;
    let target = config.targets.get(body.id).ok_or_else(|| ApiError::not_found("no such target"))?;
    let lock = config::var_dir().join("delete.lock");
    match snapshot::delete(&target.path, &body.snapshot, &lock) {
        Ok(()) => {
            monitor::log(&format!("{user} deleted {}/{}", target.path.display(), body.snapshot));
            Ok(json!({ "deleted": body.snapshot }))
        }
        Err(reason) => Err(ApiError::new(409, reason)),
    }
}

/// Sends the test notification with the channels as set in the form, saved
/// or not, so they can be tried before saving.
fn notify_test(request: &Request, user: &str) -> Result<serde_json::Value, ApiError> {
    let notifications: config::Notifications =
        serde_json::from_slice(&request.body).map_err(|e| ApiError::bad_request(format!("invalid settings: {e}")))?;
    let check = Config { notifications: notifications.clone(), ..Config::default() };
    check.validate().map_err(ApiError::bad_request)?;
    if !notifications.desktop && !notifications.webhook.enabled {
        return Err(ApiError::bad_request("no notification channel is enabled"));
    }
    monitor::send(&monitor::Event::test(user), &notifications)
        .map_err(|e| ApiError::new(502, format!("cannot notify: {e}")))?;
    Ok(json!({ "sent": true }))
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
        // no login: tells whether the package works, nothing about backups
        return Ok(cgi::json(out, 200, &health())?);
    }
    let user = crate::auth::require_admin()?;
    let config_path = Config::path();
    let mut config = Config::load(&config_path)?;

    let writes = request.method == "POST";
    if writes && !request.content_type.starts_with("application/json") {
        // a cross-site form can only post form encodings or text/plain
        return Err(ApiError::bad_request("expected application/json"));
    }

    match (request.method.as_str(), request.route.as_str()) {
        ("GET", "overview") => {
            // first run: offer what is already on the volumes
            if config.targets.is_empty() && !config_path.exists() {
                config.targets = snapshot::discover(Path::new("/")).into_iter().map(Target::new).collect();
                config.save(&config_path)?;
            }
            cgi::json(out, 200, &overview(&config, &user))?;
        }
        ("GET", "target") => cgi::json(out, 200, &target_detail(&config, request)?)?,
        ("GET", "browse") => cgi::json(out, 200, &browse::list(&snapshot_path(&config, request)?)?)?,
        ("GET", "download") => download(&config, request, out)?,
        ("GET", "settings") => cgi::json(out, 200, &config)?,
        ("POST", "settings") => cgi::json(out, 200, &save_settings(request, &config_path)?)?,
        ("POST", "discover") => cgi::json(out, 200, &discover())?,
        ("POST", "notify-test") => cgi::json(out, 200, &notify_test(request, &user)?)?,
        ("POST", "delete") => cgi::json(out, 200, &delete_snapshot(&config, request, &user)?)?,
        ("GET", "monitor") => cgi::json(out, 200, &monitor_status(&config))?,
        (_, "overview" | "target" | "browse" | "download" | "settings" | "discover" | "notify-test" | "monitor" | "delete") => {
            return Err(ApiError::new(405, "method not allowed"));
        }
        _ => return Err(ApiError::not_found("no such API")),
    }
    Ok(())
}

fn health() -> serde_json::Value {
    // SAFETY: getuid cannot fail
    let uid = unsafe { libc::getuid() };
    json!({ "version": VERSION, "uid": uid, "capabilities": crate::caps::effective() })
}

/// Serves one CGI request.
pub fn serve() -> io::Result<()> {
    let stdout = io::stdout();
    let mut out = BufWriter::with_capacity(256 * 1024, stdout.lock());
    let result = Request::from_env().map_err(ApiError::from).and_then(|request| route(&request, &mut out));
    if let Err(err) = result {
        cgi::error(&mut out, &err)?;
    }
    out.flush()
}

