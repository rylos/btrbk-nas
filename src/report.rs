//! The state of every target, as the interface shows it and the periodic
//! check judges it.

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::config::{Config, Target, Thresholds};
use crate::snapshot::{self, Group, Problem};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    #[default]
    Ok,
    Warning,
    Critical,
}

pub fn now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

/// How late the newest complete backup is, against the thresholds.
pub fn freshness(last_received: Option<i64>, now: i64, thresholds: Thresholds) -> Status {
    let Some(last) = last_received else {
        return Status::Critical;
    };
    let hours = (now - last) / 3600;
    if hours >= i64::from(thresholds.critical_hours) {
        Status::Critical
    } else if hours >= i64::from(thresholds.warning_hours) {
        Status::Warning
    } else {
        Status::Ok
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct GroupSummary {
    pub subvolume: String,
    pub count: usize,
    pub newest: Option<String>,
    pub oldest: Option<String>,
    pub last_received: Option<i64>,
    /// Interrupted receives: btrbk ignores them, they only take space.
    pub incomplete: usize,
    /// Directories named like snapshots that are not subvolumes.
    pub not_subvolume: usize,
    pub exclusive_bytes: Option<u64>,
    /// Freshness only: anomalies are reported by the counters above.
    pub status: Status,
    /// From the PC's report, when there is one.
    pub chain: Option<crate::snapshot::Chain>,
}

pub fn summarise(group: &Group, now: i64, thresholds: Thresholds) -> GroupSummary {
    let last_received = group.last_received();
    GroupSummary {
        subvolume: group.subvolume.clone(),
        count: group.subvolumes().count(),
        newest: group.subvolumes().next().map(|s| s.name.clone()),
        oldest: group.subvolumes().next_back().map(|s| s.name.clone()),
        last_received,
        incomplete: group.count(Problem::Incomplete),
        not_subvolume: group.count(Problem::NotSubvolume),
        exclusive_bytes: group.exclusive_bytes(),
        status: freshness(last_received, now, thresholds),
        chain: group.chain.clone(),
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct TargetSummary {
    pub id: usize,
    pub label: String,
    pub path: PathBuf,
    pub status: Status,
    pub notify: bool,
    pub warning_hours: u32,
    pub critical_hours: u32,
    pub last_received: Option<i64>,
    pub groups: Vec<GroupSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// The PC's report: who sent it and when, or why it cannot be read.
    pub source: Option<SourceInfo>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SourceInfo {
    pub host: String,
    pub generated: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

pub fn source_info(source: &Option<Result<crate::source::Source, String>>) -> Option<SourceInfo> {
    source.as_ref().map(|s| match s {
        Ok(s) => SourceInfo { host: s.host.clone(), generated: s.generated, error: None },
        Err(e) => SourceInfo { host: String::new(), generated: 0, error: Some(e.clone()) },
    })
}

/// Scans one target and judges it. A directory that cannot be read, or
/// whose btrfs metadata cannot be read, is critical and says why.
pub fn assess(id: usize, target: &Target, config: &Config, now: i64) -> TargetSummary {
    let thresholds = config.thresholds(target);
    let (groups, error, source) = match snapshot::scan(&target.path) {
        Ok(scan) => {
            let groups: Vec<_> = scan.groups.iter().map(|g| summarise(g, now, thresholds)).collect();
            let source = source_info(&scan.source);
            (groups, scan.metadata_error.map(|e| format!("cannot read btrfs metadata: {e}")), source)
        }
        Err(e) => (Vec::new(), Some(format!("{}: {e}", target.path.display())), None),
    };
    let status = if error.is_some() || groups.is_empty() {
        Status::Critical
    } else {
        groups.iter().map(|g| g.status).max().unwrap_or(Status::Ok)
    };
    TargetSummary {
        id,
        label: target.display_name(),
        path: target.path.clone(),
        status,
        notify: target.notify,
        warning_hours: thresholds.warning_hours,
        critical_hours: thresholds.critical_hours,
        last_received: groups.iter().filter_map(|g| g.last_received).max(),
        groups,
        error,
        source,
    }
}

pub fn assess_all(config: &Config, now: i64) -> Vec<TargetSummary> {
    config.targets.iter().enumerate().map(|(id, target)| assess(id, target, config, now)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const DEFAULT: Thresholds = Thresholds { warning_hours: 26, critical_hours: 72 };

    #[test]
    fn freshness_follows_thresholds() {
        let now = 1_000_000;
        assert_eq!(freshness(Some(now - 3600), now, DEFAULT), Status::Ok);
        assert_eq!(freshness(Some(now - 26 * 3600), now, DEFAULT), Status::Warning);
        assert_eq!(freshness(Some(now - 72 * 3600), now, DEFAULT), Status::Critical);
        assert_eq!(freshness(None, now, DEFAULT), Status::Critical);
        let weekly = Thresholds { warning_hours: 8 * 24, critical_hours: 15 * 24 };
        assert_eq!(freshness(Some(now - 72 * 3600), now, weekly), Status::Ok);
    }

    #[test]
    fn incomplete_receive_is_not_a_backup() {
        let info = crate::btrfs::SubvolInfo {
            readonly: true,
            received_uuid: Some("r".into()),
            rtime: 990_000,
            ..Default::default()
        };
        let mut group = Group {
            subvolume: "@".into(),
            snapshots: vec![snapshot::Snapshot {
                name: "@.20261026".into(),
                timestamp: "20261026".into(),
                problem: None,
                info: Some(info),
                usage: None,
                on_source: None,
                deletion: Default::default(),
            }],
            chain: None,
        };
        assert_eq!(summarise(&group, 1_000_000, DEFAULT).status, Status::Ok);
        group.snapshots[0].problem = Some(Problem::Incomplete);
        // the only snapshot being incomplete leaves no backup at all
        let summary = summarise(&group, 1_000_000, DEFAULT);
        assert_eq!(summary.status, Status::Critical);
        assert_eq!((summary.count, summary.incomplete), (1, 1));
    }
}
