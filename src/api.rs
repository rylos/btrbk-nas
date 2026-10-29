//! The JSON API behind the interface.

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
            (groups, scan.metadata_error.map(|e| t!("metadati btrfs non leggibili: {e}", "cannot read btrfs metadata: {e}")), source)
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
        serde_json::from_slice(&request.body).map_err(|e| ApiError::bad_request(t!("impostazioni non valide: {e}", "invalid settings: {e}")))?;
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
        snapshot: String,
    }
    let body: Body =
        serde_json::from_slice(&request.body).map_err(|e| ApiError::bad_request(t!("richiesta non valida: {e}", "invalid request: {e}")))?;
    let target =
        config.targets.get(body.id).ok_or_else(|| ApiError::not_found(t!("cartella di backup inesistente", "no such target")))?;
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
        serde_json::from_slice(&request.body).map_err(|e| ApiError::bad_request(t!("impostazioni non valide: {e}", "invalid settings: {e}")))?;
    let check = Config { notifications: notifications.clone(), ..Config::default() };
    check.validate().map_err(ApiError::bad_request)?;
    if !notifications.desktop && !notifications.webhook.enabled {
        return Err(ApiError::bad_request(t!("nessun canale di notifica attivo", "no notification channel is enabled")));
    }
    monitor::send(&monitor::Event::test(user), &notifications)
        .map_err(|e| ApiError::new(502, t!("notifica non inviata: {e}", "cannot notify: {e}")))?;
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
        // no login: says the CGI runs, nothing else
        return Ok(cgi::json(out, 200, &json!({ "ok": true }))?);
    }
    let user = crate::auth::require_admin()?;
    crate::caps::limit(capabilities(&request.method, &request.route))?;
    let config_path = Config::path();
    let mut config = Config::load(&config_path)?;

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
                config.notifications.language = if crate::lang::italian() { "it" } else { "en" }.into();
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
        ("GET", "monitor") => cgi::json(out, 200, &monitor_status(&config))?,
        (_, "overview" | "target" | "browse" | "download" | "settings" | "discover" | "notify-test" | "monitor" | "delete") => {
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

/// Serves one CGI request.
pub fn serve() -> io::Result<()> {
    // nothing runs with the capabilities before the login check
    crate::caps::suspend()?;
    let stdout = io::stdout();
    let mut out = BufWriter::with_capacity(256 * 1024, stdout.lock());
    let result = Request::from_env().map_err(ApiError::from).and_then(|request| {
        crate::lang::set(request.param("lang"));
        route(&request, &mut out)
    });
    if let Err(err) = result {
        cgi::error(&mut out, &err)?;
    }
    out.flush()
}

