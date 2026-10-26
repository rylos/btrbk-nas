//! Package settings: which directories hold btrbk backups, when a backup
//! counts as late, and what to notify.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

const PACKAGE: &str = "btrbk-nas";

fn yes() -> bool {
    true
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Target {
    /// Directory btrbk sends into, e.g. /volume1/backup/desktop-btrfs.
    pub path: PathBuf,
    /// Name shown in the interface, usually the sending host.
    #[serde(default)]
    pub label: String,
    /// Send notifications about this target.
    #[serde(default = "yes")]
    pub notify: bool,
    /// Overrides of the global thresholds, for a PC that is not on daily.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub warning_hours: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub critical_hours: Option<u32>,
}

impl Target {
    pub fn new(path: PathBuf) -> Self {
        Self {
            label: default_label(&path),
            path,
            notify: true,
            warning_hours: None,
            critical_hours: None,
        }
    }

    /// Name shown in the interface and in notifications.
    pub fn display_name(&self) -> String {
        if self.label.trim().is_empty() { default_label(&self.path) } else { self.label.clone() }
    }
}

/// When a backup is late, in hours since the last complete one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Thresholds {
    pub warning_hours: u32,
    pub critical_hours: u32,
}

/// An HTTP endpoint that receives the full text of each notification: a
/// Telegram bot, ntfy, Discord, a Home Assistant webhook...
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Webhook {
    pub enabled: bool,
    /// {title} and {message} are replaced, URL-encoded. With an empty body
    /// the URL is called with GET (Telegram: .../sendMessage?chat_id=..&text={message}).
    pub url: String,
    /// When set, sent with POST as application/json; {title} and {message}
    /// are replaced, JSON-escaped. Discord: {"content": "{message}"}
    pub body: String,
}

/// Which events are notified, and where. Third-party packages cannot
/// register events in DSM's notification rules (DSM refuses the sysnotify
/// worker to unsigned packages), so the channels are the DSM desktop and an
/// optional webhook.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Notifications {
    /// Master switch for the periodic check.
    pub enabled: bool,
    /// How often the backups are checked.
    pub interval_minutes: u32,
    /// A subvolume becomes late.
    pub late: bool,
    /// A subvolume becomes critical.
    pub critical: bool,
    /// A late or critical subvolume is up to date again.
    pub recovered: bool,
    /// New interrupted receives appear.
    pub incomplete: bool,
    /// A target cannot be read (missing directory, missing permissions).
    pub errors: bool,
    /// The PC reports no snapshot in common with the NAS: btrbk's next send
    /// will transfer everything.
    pub full_send: bool,
    /// Repeat a late/critical/error notification this often while it lasts;
    /// 0 notifies once.
    pub reminder_hours: u32,
    /// Notifications on the DSM desktop.
    pub desktop: bool,
    /// Who gets them: a user name, or @group.
    pub recipient: String,
    pub webhook: Webhook,
    /// Language of the webhook texts: "it" or "en".
    pub language: String,
}

impl Default for Notifications {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_minutes: 60,
            late: true,
            critical: true,
            recovered: true,
            incomplete: true,
            errors: true,
            full_send: true,
            reminder_hours: 24,
            desktop: true,
            recipient: "@administrators".into(),
            webhook: Webhook::default(),
            language: "it".into(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Config {
    pub targets: Vec<Target>,
    /// A subvolume whose newest backup is older than this is late.
    pub warning_hours: u32,
    /// ...and this much older is critical.
    pub critical_hours: u32,
    pub notifications: Notifications,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            targets: Vec::new(),
            warning_hours: 26,
            critical_hours: 72,
            notifications: Notifications::default(),
        }
    }
}

/// DSM gives every package a private, writable var directory.
pub fn var_dir() -> PathBuf {
    std::env::var_os("SYNOPKG_PKGVAR")
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new("/var/packages").join(PACKAGE).join("var"))
}

/// Writes atomically: a crash never leaves half a file behind.
pub fn write_atomic(path: &Path, data: &[u8]) -> io::Result<()> {
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    fs::write(&tmp, data)?;
    fs::rename(&tmp, path)
}

impl Config {
    pub fn path() -> PathBuf {
        var_dir().join("config.json")
    }

    /// The saved settings; defaults when there are none yet.
    pub fn load(path: &Path) -> io::Result<Self> {
        match fs::read(path) {
            Ok(data) => serde_json::from_slice(&data).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e)),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e),
        }
    }

    /// Saved readable by the package user only: a webhook URL can hold a
    /// bot token.
    pub fn save(&self, path: &Path) -> io::Result<()> {
        write_atomic(path, &serde_json::to_vec_pretty(self).map_err(io::Error::other)?)?;
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))
    }

    /// The thresholds that apply to `target`: its own, or the global ones.
    pub fn thresholds(&self, target: &Target) -> Thresholds {
        Thresholds {
            warning_hours: target.warning_hours.unwrap_or(self.warning_hours),
            critical_hours: target.critical_hours.unwrap_or(self.critical_hours),
        }
    }

    /// Checks settings coming from the interface.
    pub fn validate(&self) -> Result<(), String> {
        fn check(t: Thresholds, what: &str) -> Result<(), String> {
            if t.warning_hours == 0 || t.critical_hours <= t.warning_hours {
                return Err(format!("{what}: critical hours must be greater than late hours, both above zero"));
            }
            Ok(())
        }
        check(Thresholds { warning_hours: self.warning_hours, critical_hours: self.critical_hours }, "thresholds")?;
        let n = &self.notifications;
        if !(5..=1440).contains(&n.interval_minutes) {
            return Err("the check interval must be between 5 and 1440 minutes".into());
        }
        if n.reminder_hours > 24 * 30 {
            return Err("reminders can be at most 720 hours apart".into());
        }
        let name = n.recipient.strip_prefix('@').unwrap_or(&n.recipient);
        if n.desktop && (name.is_empty() || !name.chars().all(|c| c.is_alphanumeric() || "._- ".contains(c))) {
            return Err("the desktop recipient must be a user name or @group".into());
        }
        if n.webhook.enabled && !(n.webhook.url.starts_with("https://") || n.webhook.url.starts_with("http://")) {
            return Err("the webhook URL must start with https:// or http://".into());
        }
        if !matches!(n.language.as_str(), "it" | "en") {
            return Err("language must be it or en".into());
        }
        for target in &self.targets {
            let path = &target.path;
            let under_volume = path
                .components()
                .nth(1)
                .and_then(|c| c.as_os_str().to_str())
                .is_some_and(|first| first.starts_with("volume"));
            if !path.is_absolute() || !under_volume || path.components().any(|c| c.as_os_str() == "..") {
                return Err(format!("{}: a target must be a directory under /volumeN", path.display()));
            }
            if self.targets.iter().filter(|t| t.path == *path).count() > 1 {
                return Err(format!("{}: listed twice", path.display()));
            }
            check(self.thresholds(target), &target.display_name())?;
        }
        Ok(())
    }
}

/// Label for a newly found target: the directory name without btrbk's
/// customary "-btrfs" suffix.
pub fn default_label(path: &Path) -> String {
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    name.strip_suffix("-btrfs").unwrap_or(&name).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target(path: &str) -> Target {
        Target::new(path.into())
    }

    #[test]
    fn validates_paths_and_thresholds() {
        let mut config = Config { targets: vec![target("/volume1/backup/desktop-btrfs")], ..Config::default() };
        assert!(config.validate().is_ok());
        config.targets.push(target("/etc"));
        assert!(config.validate().is_err());
        config.targets[1] = target("/volume1/../etc");
        assert!(config.validate().is_err());
        config.targets[1] = target("/volume1/backup/desktop-btrfs");
        assert!(config.validate().is_err());
        config.targets.pop();
        config.critical_hours = 10;
        assert!(config.validate().is_err());
    }

    #[test]
    fn target_thresholds_override_global_ones() {
        let mut config = Config { targets: vec![target("/volume1/b")], ..Config::default() };
        config.targets[0].warning_hours = Some(100);
        // 100 late but 72 critical: inconsistent once merged
        assert!(config.validate().is_err());
        config.targets[0].critical_hours = Some(200);
        assert!(config.validate().is_ok());
        assert_eq!(config.thresholds(&config.targets[0]), Thresholds { warning_hours: 100, critical_hours: 200 });
    }

    #[test]
    fn checks_notification_settings() {
        let mut config = Config::default();
        config.notifications.interval_minutes = 1;
        assert!(config.validate().is_err());
        config.notifications.interval_minutes = 30;
        assert!(config.validate().is_ok());
    }

    #[test]
    fn checks_channels() {
        let mut config = Config::default();
        config.notifications.recipient = "@administrators; rm -rf /".into();
        assert!(config.validate().is_err());
        config.notifications.recipient = "alice".into();
        assert!(config.validate().is_ok());
        config.notifications.webhook = Webhook { enabled: true, url: "file:///etc/passwd".into(), body: String::new() };
        assert!(config.validate().is_err());
        config.notifications.webhook.url = "https://ntfy.sh/btrbk".into();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn reads_files_of_older_versions() {
        let config: Config = serde_json::from_str(r#"{"targets":[{"path":"/volume1/b"}],"warning_hours":30}"#).unwrap();
        assert_eq!(config.warning_hours, 30);
        assert_eq!(config.critical_hours, 72);
        assert!(config.targets[0].notify);
        assert_eq!(config.targets[0].label, "");
        assert_eq!(config.targets[0].display_name(), "b");
        assert!(config.notifications.enabled);
    }

    #[test]
    fn labels_from_directory_name() {
        assert_eq!(default_label(Path::new("/volume1/backup/desktop-btrfs")), "desktop");
        assert_eq!(default_label(Path::new("/volume1/srv")), "srv");
    }
}
