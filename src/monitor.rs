//! The periodic check: compares every target with what was last seen and
//! sends DSM notifications on changes only, with optional reminders.
//!
//! It runs in `btrbk-nas daemon`, started by the package's
//! start-stop-status: stopping the package in Package Center stops the
//! check. The daemon reloads the settings every minute, so a change made in
//! the interface applies without a restart.

use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread::sleep;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::config::{self, Config, Notifications, Webhook};
use crate::report::{self, Status, TargetSummary};

/// Opens the app when the desktop notification is clicked.
const APP_CLASS: &str = "SYNO.SDS.BtrbkNas.Instance";
/// Leaves the NAS time to finish booting before the first check.
const FIRST_CHECK_DELAY: Duration = Duration::from_secs(120);
const TICK: Duration = Duration::from_secs(60);
const MAX_LOG: u64 = 512 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Late,
    Critical,
    Recovered,
    Incomplete,
    Error,
    FullSend,
    Test,
}

impl Kind {
    fn wanted(self, n: &Notifications) -> bool {
        match self {
            Kind::Late => n.late,
            Kind::Critical => n.critical,
            Kind::Recovered => n.recovered,
            Kind::Incomplete => n.incomplete,
            Kind::Error => n.errors,
            Kind::FullSend => n.full_send,
            Kind::Test => true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Event {
    pub kind: Kind,
    pub target: String,
    pub subvolume: String,
    /// Hours since the last complete backup, when there is one.
    pub hours: Option<i64>,
    /// Newly found incomplete receives.
    pub count: usize,
    pub detail: String,
    pub reminder: bool,
}

impl Event {
    /// The test notification; `sender` names who asked for it.
    pub fn test(sender: &str) -> Self {
        Self {
            kind: Kind::Test,
            target: sender.to_string(),
            subvolume: String::new(),
            hours: None,
            count: 0,
            detail: String::new(),
            reminder: false,
        }
    }
}

/// What the last check saw, per target and subvolume.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Seen {
    pub status: Status,
    pub incomplete: usize,
    /// The PC reported no snapshot in common.
    #[serde(default)]
    pub full_send: bool,
    /// Set while the target cannot be read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// When the current problem was last notified (0: never).
    #[serde(default)]
    pub notified_at: i64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct State {
    pub last_check: i64,
    #[serde(default)]
    pub seen: BTreeMap<String, Seen>,
}

impl State {
    pub fn path() -> PathBuf {
        config::var_dir().join("monitor-state.json")
    }

    pub fn load(path: &Path) -> Self {
        fs::read(path).ok().and_then(|data| serde_json::from_slice(&data).ok()).unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        config::write_atomic(path, &serde_json::to_vec_pretty(self).map_err(io::Error::other)?)
    }
}

fn key(target: &TargetSummary, subvolume: &str) -> String {
    format!("{}\t{subvolume}", target.path.display())
}

/// Compares the targets with the previous state. Returns the new state and
/// the events to send, already filtered by the settings.
pub fn evaluate(previous: &State, targets: &[TargetSummary], config: &Config, now: i64) -> (State, Vec<Event>) {
    let n = &config.notifications;
    let reminder_due = |at: i64| n.reminder_hours > 0 && at > 0 && now - at >= i64::from(n.reminder_hours) * 3600;
    let mut state = State { last_check: now, seen: BTreeMap::new() };
    let mut events = Vec::new();

    for target in targets.iter().filter(|t| t.notify) {
        let mut emit = |kind: Kind, subvolume: &str, hours: Option<i64>, count: usize, detail: String, reminder: bool| {
            if kind.wanted(n) {
                events.push(Event {
                    kind,
                    target: target.label.clone(),
                    subvolume: subvolume.to_string(),
                    hours,
                    count,
                    detail,
                    reminder,
                });
                true
            } else {
                false
            }
        };

        // a target that cannot be read: one error, no per-subvolume noise
        if let Some(error) = &target.error {
            let k = key(target, "");
            let before = previous.seen.get(&k).cloned().unwrap_or_default();
            let mut seen = Seen { status: Status::Critical, error: Some(error.clone()), ..before.clone() };
            if before.error.is_none() {
                seen.notified_at = if emit(Kind::Error, "", None, 0, error.clone(), false) { now } else { 0 };
            } else if reminder_due(before.notified_at) && emit(Kind::Error, "", None, 0, error.clone(), true) {
                seen.notified_at = now;
            }
            state.seen.insert(k, seen);
            continue;
        }

        for group in &target.groups {
            let k = key(target, &group.subvolume);
            let before = previous.seen.get(&k).cloned().unwrap_or_default();
            let hours = group.last_received.map(|last| (now - last) / 3600);
            let full_send = group.chain.as_ref().is_some_and(|c| c.full_send);
            let mut seen =
                Seen { status: group.status, incomplete: group.incomplete, full_send, error: None, notified_at: 0 };

            let kind = match group.status {
                Status::Warning => Some(Kind::Late),
                Status::Critical => Some(Kind::Critical),
                Status::Ok => None,
            };
            if group.status > before.status {
                // got worse: late, or straight to critical
                if let Some(kind) = kind
                    && emit(kind, &group.subvolume, hours, 0, String::new(), false)
                {
                    seen.notified_at = now;
                }
            } else if group.status == Status::Ok && before.status != Status::Ok {
                emit(Kind::Recovered, &group.subvolume, hours, 0, String::new(), false);
            } else if let Some(kind) = kind {
                // still a problem (or slightly better): keep the reminder clock
                seen.notified_at = before.notified_at;
                if reminder_due(before.notified_at) && emit(kind, &group.subvolume, hours, 0, String::new(), true) {
                    seen.notified_at = now;
                }
            }

            if full_send && !before.full_send {
                emit(Kind::FullSend, &group.subvolume, hours, 0, String::new(), false);
            }
            if group.incomplete > before.incomplete {
                emit(Kind::Incomplete, &group.subvolume, hours, group.incomplete - before.incomplete, String::new(), false);
            }
            state.seen.insert(k, seen);
        }
    }
    (state, events)
}

const SYNODSMNOTIFY: &str = "/usr/syno/bin/synodsmnotify";
const CURL: &str = "/usr/bin/curl";

impl Kind {
    /// Key of the desktop text in ui/texts/<lang>/strings, section [app].
    fn text_key(self) -> &'static str {
        match self {
            Kind::Late => "late_msg",
            Kind::Critical => "critical_msg",
            Kind::Recovered => "recovered_msg",
            Kind::Incomplete => "incomplete_msg",
            Kind::Error => "error_msg",
            Kind::FullSend => "full_send_msg",
            Kind::Test => "test_msg",
        }
    }

    fn level(self) -> &'static str {
        match self {
            Kind::Critical | Kind::Error => "error",
            Kind::Late | Kind::Incomplete | Kind::FullSend | Kind::Test => "warn",
            Kind::Recovered => "info",
        }
    }
}

impl Event {
    /// The full text, for the webhook. The desktop only takes the package's
    /// fixed translated strings, so it cannot carry these details.
    pub fn text(&self, language: &str) -> String {
        let it = language == "it";
        let hours = self.hours.map(|h| h.to_string()).unwrap_or_else(|| "?".into());
        let (s, t) = (&self.subvolume, &self.target);
        let text = match (self.kind, it) {
            (Kind::Late, true) => format!("Backup di {s} da {t} in ritardo: l'ultimo completo è di {hours} ore fa."),
            (Kind::Late, false) => format!("Backup of {s} from {t} is late: the last complete one is {hours} hours old."),
            (Kind::Critical, true) => format!("Nessun backup completo di {s} da {t} da {hours} ore."),
            (Kind::Critical, false) => format!("No complete backup of {s} from {t} for {hours} hours."),
            (Kind::Recovered, true) => format!("Il backup di {s} da {t} è di nuovo aggiornato."),
            (Kind::Recovered, false) => format!("The backup of {s} from {t} is up to date again."),
            (Kind::Incomplete, true) => {
                format!("{} nuove ricezioni incomplete di {s} da {t}: btrbk le rimanda al prossimo giro.", self.count)
            }
            (Kind::Incomplete, false) => {
                format!("{} new incomplete receive(s) of {s} from {t}: btrbk sends them again next time.", self.count)
            }
            (Kind::Error, true) => format!("I backup di {t} non sono leggibili: {}", self.detail),
            (Kind::Error, false) => format!("The backups of {t} cannot be read: {}", self.detail),
            (Kind::FullSend, true) => format!(
                "{t} non ha più snapshot di {s} in comune con il NAS: il prossimo invio di btrbk sarà completo."
            ),
            (Kind::FullSend, false) => {
                format!("{t} has no snapshot of {s} in common with the NAS any more: btrbk's next send will be a full one.")
            }
            (Kind::Test, true) => format!("Notifica di prova inviata da {t}."),
            (Kind::Test, false) => format!("Test notification sent by {t}."),
        };
        match (self.reminder, it) {
            (true, true) => format!("{text} (promemoria)"),
            (true, false) => format!("{text} (reminder)"),
            _ => text,
        }
    }
}

/// A notification on the DSM desktop. Title and text must be translated
/// strings of the package (listed in preloadTexts of ui/config): DSM refuses
/// free text.
fn send_desktop(event: &Event, recipient: &str) -> io::Result<()> {
    let status = Command::new(SYNODSMNOTIFY)
        .args(["-c", APP_CLASS, "-l", event.kind.level(), "-e", "false", "-b", "false", recipient])
        .arg("btrbk-nas:app:title")
        .arg(format!("btrbk-nas:app:{}", event.kind.text_key()))
        .status()?;
    if status.success() { Ok(()) } else { Err(io::Error::other(format!("{SYNODSMNOTIFY} exited with {status}"))) }
}

fn url_encode(text: &str) -> String {
    crate::cgi::percent_encode(text)
}

fn json_escape(text: &str) -> String {
    let quoted = serde_json::to_string(text).unwrap_or_default();
    quoted[1..quoted.len() - 1].to_string()
}

/// Fills {title} and {message} in the webhook URL and body.
pub fn webhook_request(webhook: &Webhook, title: &str, message: &str) -> (String, Option<String>) {
    let url = webhook.url.replace("{title}", &url_encode(title)).replace("{message}", &url_encode(message));
    let body = (!webhook.body.trim().is_empty()).then(|| {
        webhook.body.replace("{title}", &json_escape(title)).replace("{message}", &json_escape(message))
    });
    (url, body)
}

/// A quoted string of a curl config file.
fn curl_quote(text: &str) -> String {
    let mut quoted = String::from("\"");
    for c in text.chars() {
        match c {
            '\\' => quoted.push_str("\\\\"),
            '"' => quoted.push_str("\\\""),
            '\n' => quoted.push_str("\\n"),
            '\r' => quoted.push_str("\\r"),
            '\t' => quoted.push_str("\\t"),
            c => quoted.push(c),
        }
    }
    quoted.push('"');
    quoted
}

/// curl's config, read from stdin: on the command line a token in the URL
/// would show in ps and /proc/*/cmdline.
fn curl_config(url: &str, body: Option<&str>) -> String {
    let mut config = format!("url = {}\n", curl_quote(url));
    if let Some(body) = body {
        // data-raw, not data-binary: a body starting with @ is not a file name
        config.push_str("request = \"POST\"\nheader = \"Content-Type: application/json\"\n");
        config.push_str(&format!("data-raw = {}\n", curl_quote(body)));
    }
    config
}

fn send_webhook(event: &Event, n: &Notifications) -> io::Result<()> {
    let (url, body) = webhook_request(&n.webhook, "btrbk Backups", &event.text(&n.language));
    let mut child = Command::new(CURL)
        .args(["--silent", "--show-error", "--fail", "--max-time", "20", "--output", "/dev/null", "--config", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(curl_config(&url, body.as_deref()).as_bytes())?;
    }
    let output = child.wait_with_output()?;
    if output.status.success() {
        Ok(())
    } else {
        let error = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(io::Error::other(format!("webhook: {error}")))
    }
}

/// Sends an event on every enabled channel. Reports the channels that
/// failed, after trying them all.
pub fn send(event: &Event, n: &Notifications) -> io::Result<()> {
    let mut errors = Vec::new();
    if n.desktop
        && let Err(e) = send_desktop(event, &n.recipient)
    {
        errors.push(format!("desktop: {e}"));
    }
    if n.webhook.enabled
        && let Err(e) = send_webhook(event, n)
    {
        errors.push(e.to_string());
    }
    if errors.is_empty() { Ok(()) } else { Err(io::Error::other(errors.join("; "))) }
}

/// One check: assess, compare, notify, save. With `dry_run` nothing is sent
/// or saved.
pub fn check(config: &Config, dry_run: bool) -> io::Result<Vec<Event>> {
    // the details of an error travel in the webhook text: in its language
    crate::lang::set(Some(&config.notifications.language));
    let now = report::now();
    let targets = report::assess_all(config, now);
    let path = State::path();
    let (state, events) = evaluate(&State::load(&path), &targets, config, now);
    if !dry_run {
        for event in &events {
            if let Err(e) = send(event, &config.notifications) {
                log(&format!("cannot notify {:?} {}/{}: {e}", event.kind, event.target, event.subvolume));
            }
        }
        state.save(&path)?;
    }
    Ok(events)
}

pub fn log_path() -> PathBuf {
    config::var_dir().join("monitor.log")
}

pub fn log(message: &str) {
    let path = log_path();
    if fs::metadata(&path).is_ok_and(|m| m.len() > MAX_LOG) {
        let _ = fs::rename(&path, path.with_extension("log.1"));
    }
    if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(&path) {
        let stamp = chrono::DateTime::from_timestamp(report::now(), 0).map(|t| t.to_rfc3339()).unwrap_or_default();
        let _ = writeln!(file, "{stamp} {message}");
    }
}

/// Without capabilities every target looks unreadable: rather than notify
/// errors that only mean "setup has not run yet", wait for setup, then
/// re-exec to pick up the capabilities (they are granted at exec time).
fn wait_for_capabilities() {
    if crate::caps::granted() {
        return;
    }
    log("capabilities missing: waiting for 'btrbk-nas setup'");
    let Ok(binary) = std::env::current_exe() else { return };
    while !crate::caps::on_file(&binary) {
        sleep(TICK);
    }
    log("capabilities granted: restarting");
    use std::os::unix::process::CommandExt;
    let error = Command::new(&binary).arg("daemon").exec();
    log(&format!("cannot restart: {error}"));
}

/// The daemon loop. Never returns.
pub fn run() -> ! {
    // checking and notifying never delete: no CAP_DAC_OVERRIDE
    if let Err(e) = crate::caps::suspend().and_then(|()| crate::caps::limit(crate::caps::READ)) {
        log(&format!("cannot drop capabilities, not starting: {e}"));
        std::process::exit(1);
    }
    log(&format!("started, version {}", crate::api::VERSION));
    sleep(FIRST_CHECK_DELAY);
    loop {
        wait_for_capabilities();
        match Config::load(&Config::path()) {
            Ok(config) if config.notifications.enabled => {
                let due = State::load(&State::path()).last_check
                    + i64::from(config.notifications.interval_minutes) * 60;
                if report::now() >= due {
                    match check(&config, false) {
                        Ok(events) => {
                            for e in &events {
                                log(&format!("notified {:?} {}/{} {}", e.kind, e.target, e.subvolume, e.detail));
                            }
                        }
                        Err(e) => log(&format!("check failed: {e}")),
                    }
                }
            }
            Ok(_) => {}
            Err(e) => log(&format!("cannot read settings: {e}")),
        }
        sleep(TICK);
    }
}

pub fn pid_path() -> PathBuf {
    config::var_dir().join("monitor.pid")
}

/// Whether the daemon runs, from its pid file.
pub fn running() -> bool {
    let Some(pid) = fs::read_to_string(pid_path()).ok().and_then(|p| p.trim().parse::<i32>().ok()) else {
        return false;
    };
    // SAFETY: signal 0 only checks that the process exists
    pid > 0 && unsafe { libc::kill(pid, 0) } == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::GroupSummary;

    const HOUR: i64 = 3600;
    const NOW: i64 = 1_000_000 * HOUR;

    fn group(subvolume: &str, status: Status, hours_ago: i64, incomplete: usize) -> GroupSummary {
        GroupSummary {
            subvolume: subvolume.into(),
            count: 5,
            newest: None,
            oldest: None,
            last_received: Some(NOW - hours_ago * HOUR),
            incomplete,
            not_subvolume: 0,
            exclusive_bytes: None,
            status,
            chain: None,
        }
    }

    fn target(groups: Vec<GroupSummary>) -> TargetSummary {
        TargetSummary {
            id: 0,
            label: "PC".into(),
            path: "/volume2/backup/PC-btrfs".into(),
            status: Status::Ok,
            notify: true,
            warning_hours: 26,
            critical_hours: 72,
            last_received: None,
            groups,
            error: None,
            source: None,
        }
    }

    fn kinds(events: &[Event]) -> Vec<(Kind, bool)> {
        events.iter().map(|e| (e.kind, e.reminder)).collect()
    }

    #[test]
    fn notifies_changes_only() {
        let config = Config::default();
        let ok = [target(vec![group("@", Status::Ok, 2, 0)])];
        let late = [target(vec![group("@", Status::Warning, 30, 0)])];
        let critical = [target(vec![group("@", Status::Critical, 80, 0)])];

        let (s, e) = evaluate(&State::default(), &ok, &config, NOW);
        assert!(e.is_empty());
        let (s, e) = evaluate(&s, &late, &config, NOW);
        assert_eq!(kinds(&e), [(Kind::Late, false)]);
        assert_eq!(e[0].hours, Some(30));
        // same state an hour later: silence
        let (s, e) = evaluate(&s, &late, &config, NOW + HOUR);
        assert!(e.is_empty());
        let (s, e) = evaluate(&s, &critical, &config, NOW + 2 * HOUR);
        assert_eq!(kinds(&e), [(Kind::Critical, false)]);
        let (_, e) = evaluate(&s, &ok, &config, NOW + 3 * HOUR);
        assert_eq!(kinds(&e), [(Kind::Recovered, false)]);
    }

    #[test]
    fn reminds_while_the_problem_lasts() {
        let mut config = Config::default();
        let late = [target(vec![group("@", Status::Warning, 30, 0)])];
        let (s, _) = evaluate(&State::default(), &late, &config, NOW);
        let (s, e) = evaluate(&s, &late, &config, NOW + 23 * HOUR);
        assert!(e.is_empty());
        let (s, e) = evaluate(&s, &late, &config, NOW + 24 * HOUR);
        assert_eq!(kinds(&e), [(Kind::Late, true)]);
        // the clock restarts from the reminder
        let (_, e) = evaluate(&s, &late, &config, NOW + 30 * HOUR);
        assert!(e.is_empty());

        config.notifications.reminder_hours = 0;
        let (s, _) = evaluate(&State::default(), &late, &config, NOW);
        let (_, e) = evaluate(&s, &late, &config, NOW + 1000 * HOUR);
        assert!(e.is_empty());
    }

    #[test]
    fn respects_every_switch() {
        let late = [target(vec![group("@", Status::Warning, 30, 2)])];
        let mut config = Config::default();
        let (_, e) = evaluate(&State::default(), &late, &config, NOW);
        assert_eq!(kinds(&e), [(Kind::Late, false), (Kind::Incomplete, false)]);
        assert_eq!(e[1].count, 2);

        config.notifications.late = false;
        config.notifications.incomplete = false;
        let (s, e) = evaluate(&State::default(), &late, &config, NOW);
        assert!(e.is_empty());
        // switched off is not "notified": no reminder either
        assert_eq!(s.seen.values().next().unwrap().notified_at, 0);

        let mut muted = late.clone();
        muted[0].notify = false;
        let (s, e) = evaluate(&State::default(), &muted, &Config::default(), NOW);
        assert!(e.is_empty());
        assert!(s.seen.is_empty());
    }

    #[test]
    fn new_incomplete_receives_only() {
        let config = Config::default();
        let one = [target(vec![group("@", Status::Ok, 2, 1)])];
        let two = [target(vec![group("@", Status::Ok, 2, 2)])];
        let (s, e) = evaluate(&State::default(), &one, &config, NOW);
        assert_eq!(e[0].count, 1);
        let (s, e) = evaluate(&s, &one, &config, NOW + HOUR);
        assert!(e.is_empty());
        let (_, e) = evaluate(&s, &two, &config, NOW + 2 * HOUR);
        assert_eq!((e[0].kind, e[0].count), (Kind::Incomplete, 1));
    }

    #[test]
    fn fills_webhook_templates() {
        let webhook = Webhook {
            enabled: true,
            url: "https://api.telegram.org/botX/sendMessage?chat_id=1&text={message}".into(),
            body: String::new(),
        };
        let (url, body) = webhook_request(&webhook, "t", "Backup di @home: 30 ore");
        assert_eq!(url, "https://api.telegram.org/botX/sendMessage?chat_id=1&text=Backup%20di%20%40home%3A%2030%20ore");
        assert_eq!(body, None);

        let webhook = Webhook { body: r#"{"content": "{title}: {message}"}"#.into(), ..webhook };
        let (_, body) = webhook_request(&webhook, "btrbk", "a \"quoted\" line\nnext");
        let parsed: serde_json::Value = serde_json::from_str(&body.unwrap()).unwrap();
        assert_eq!(parsed["content"], "btrbk: a \"quoted\" line\nnext");
    }

    #[test]
    fn quotes_the_curl_config() {
        assert_eq!(curl_config("https://x/?a=\"1\"\\", None), "url = \"https://x/?a=\\\"1\\\"\\\\\"\n");
        let config = curl_config("https://x/", Some("{\"m\": \"@a\\nb\"}"));
        assert!(config.contains("request = \"POST\"\n"));
        assert!(config.ends_with("data-raw = \"{\\\"m\\\": \\\"@a\\\\nb\\\"}\"\n"));
    }

    #[test]
    fn webhook_text_has_the_details() {
        let event = Event {
            kind: Kind::Late,
            target: "desktop".into(),
            subvolume: "@home".into(),
            hours: Some(30),
            count: 0,
            detail: String::new(),
            reminder: true,
        };
        assert_eq!(event.text("it"), "Backup di @home da desktop in ritardo: l'ultimo completo è di 30 ore fa. (promemoria)");
        assert!(event.text("en").starts_with("Backup of @home from desktop is late"));
    }

    #[test]
    fn warns_once_about_a_full_send() {
        let config = Config::default();
        let mut broken = group("@games", Status::Ok, 2, 0);
        broken.chain = Some(crate::snapshot::Chain { next_parent: None, pending: vec![], full_send: true });
        let targets = [target(vec![broken])];
        let (s, e) = evaluate(&State::default(), &targets, &config, NOW);
        assert_eq!(kinds(&e), [(Kind::FullSend, false)]);
        assert!(e[0].text("it").contains("sarà completo"));
        let (_, e) = evaluate(&s, &targets, &config, NOW + HOUR);
        assert!(e.is_empty());
    }

    #[test]
    fn unreadable_target_is_one_error() {
        let config = Config::default();
        let mut broken = target(vec![]);
        broken.error = Some("Permission denied".into());
        let (s, e) = evaluate(&State::default(), &[broken.clone()], &config, NOW);
        assert_eq!(kinds(&e), [(Kind::Error, false)]);
        let (s, e) = evaluate(&s, &[broken.clone()], &config, NOW + HOUR);
        assert!(e.is_empty());
        let (_, e) = evaluate(&s, &[broken], &config, NOW + 25 * HOUR);
        assert_eq!(kinds(&e), [(Kind::Error, true)]);
    }
}
