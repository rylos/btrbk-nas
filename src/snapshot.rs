//! btrbk snapshot names and the state of the snapshots a target directory
//! holds. Name parsing is the one of btrbk_tui, kept identical on purpose.

use std::collections::HashMap;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

use chrono::{NaiveDate, NaiveDateTime};
use serde::Serialize;

use crate::btrfs::{self, SUBVOL_TOP_INO, SubvolInfo, Usage};

/// Splits "@home.20260803T0000" or "home.20260803T0000" into subvolume name
/// and timestamp. The timestamp never contains a dot, the subvolume name
/// might. A name counts as a snapshot only when what follows the last dot is
/// a btrbk timestamp.
pub fn split_snapshot_name(name: &str) -> Option<(&str, &str)> {
    let (prefix, timestamp) = name.rsplit_once('.')?;
    (!prefix.is_empty() && parse_btrbk_timestamp(timestamp).is_some()).then_some((prefix, timestamp))
}

/// Parses the timestamp btrbk puts in snapshot names, in any of its
/// timestamp_format flavours (short, long, long-iso), with the optional "_N"
/// suffix btrbk adds when a name is already taken.
pub fn parse_btrbk_timestamp(timestamp: &str) -> Option<NaiveDateTime> {
    // legacy "YYYYMMDD_HHMMSS" names: here the underscore is not a "_N" suffix
    if let Ok(dt) = NaiveDateTime::parse_from_str(timestamp, "%Y%m%d_%H%M%S") {
        return Some(dt);
    }

    let base = timestamp.split('_').next()?;
    // long-iso carries a UTC offset: the name already is local time, drop it
    let base = base.split(['+', '-']).next()?;

    match base.len() {
        8 => NaiveDate::parse_from_str(base, "%Y%m%d").ok()?.and_hms_opt(0, 0, 0),
        13 => NaiveDateTime::parse_from_str(base, "%Y%m%dT%H%M").ok(),
        15 => NaiveDateTime::parse_from_str(base, "%Y%m%dT%H%M%S").ok(),
        _ => None,
    }
}

/// Something wrong with one snapshot.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Problem {
    /// No received UUID, or still writable: `btrfs receive` was interrupted.
    /// btrbk calls these "garbled" and will not use them as a parent.
    Incomplete,
    /// A directory with a snapshot name that is not a subvolume.
    NotSubvolume,
}

#[derive(Clone, Debug, Serialize)]
pub struct Snapshot {
    pub name: String,
    /// The name's timestamp, local time of the sending host.
    pub timestamp: String,
    pub problem: Option<Problem>,
    pub info: Option<SubvolInfo>,
    pub usage: Option<Usage>,
    /// Whether the sending PC still holds this snapshot; None without a
    /// report from the PC.
    pub on_source: Option<bool>,
    /// When the PC took it (unix seconds), to the second; None unless the
    /// PC's report says.
    pub created: Option<i64>,
    /// Whether it may be deleted from the NAS, and why not.
    pub deletion: Deletion,
}

/// Whether a snapshot may be deleted from the NAS without hurting btrbk's
/// incremental chain.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Deletion {
    /// Older than the common parent: the chain no longer needs it.
    Allowed,
    /// An interrupted receive old enough not to be one still running:
    /// btrbk never uses it.
    AllowedIncomplete,
    /// The common parent: deleting it would force a full send.
    Parent,
    /// Newer than the common parent.
    Newer,
    /// The PC sends no report of its snapshots: the parent is unknown.
    #[default]
    ParentUnknown,
    /// The PC reports nothing in common: the chain is already broken, the
    /// next send is a full one; left alone, like the purge scripts do.
    NoCommonParent,
    /// An interrupted receive less than a day old: it may still be running.
    Receiving,
    /// Not a subvolume: not btrbk's, never touched.
    NotSubvolume,
}

impl Deletion {
    pub fn allowed(self) -> bool {
        matches!(self, Deletion::Allowed | Deletion::AllowedIncomplete)
    }

    /// Why it may not go, for whoever asked to delete it.
    pub fn refusal(self) -> String {
        use crate::lang::t;
        match self {
            Deletion::Allowed | Deletion::AllowedIncomplete => t!("si può cancellare", "it may be deleted"),
            Deletion::Parent => t!(
                "è il parent in comune con il PC: cancellarlo renderebbe completo il prossimo invio",
                "it is the parent in common with the PC: deleting it would make the next send a full one"
            ),
            Deletion::Newer => t!("è più recente del parent in comune", "it is newer than the parent in common"),
            Deletion::ParentUnknown => t!(
                "il PC non manda l'elenco dei suoi snapshot: il parent non è noto",
                "the PC sends no list of its snapshots: the parent is unknown"
            ),
            Deletion::NoCommonParent => t!(
                "nessuno snapshot in comune con il PC: si lascia tutto com'è",
                "no snapshot in common with the PC: everything is left alone"
            ),
            Deletion::Receiving => t!(
                "ricezione interrotta da meno di un giorno: potrebbe essere ancora in corso",
                "an interrupted receive less than a day old: it may still be running"
            ),
            Deletion::NotSubvolume => t!("non è un subvolume", "not a subvolume"),
        }
    }
}

/// An interrupted receive younger than this may be one still in progress: a
/// full send of a large subvolume takes hours.
pub const RECEIVE_GRACE_SECS: i64 = 24 * 3600;

/// Decides, for every snapshot, whether it may be deleted.
pub fn mark_deletion(groups: &mut [Group], now: i64) {
    for group in groups {
        let parent = group.chain.as_ref().map(|c| c.next_parent.clone());
        let parent_index =
            parent.as_ref().and_then(|p| p.as_ref()).and_then(|p| group.snapshots.iter().position(|s| &s.name == p));
        for (i, snap) in group.snapshots.iter_mut().enumerate() {
            snap.deletion = match (snap.problem, &parent, parent_index) {
                (Some(Problem::NotSubvolume), _, _) => Deletion::NotSubvolume,
                (Some(Problem::Incomplete), _, _) => {
                    let started = snap.info.as_ref().map_or(now, |i| i.otime);
                    if now - started >= RECEIVE_GRACE_SECS { Deletion::AllowedIncomplete } else { Deletion::Receiving }
                }
                (None, None, _) => Deletion::ParentUnknown,
                (None, Some(None), _) => Deletion::NoCommonParent,
                (None, Some(Some(_)), Some(p)) if i < p => Deletion::Newer,
                (None, Some(Some(_)), Some(p)) if i == p => Deletion::Parent,
                (None, Some(Some(_)), Some(_)) => Deletion::Allowed,
                // the parent is always one of the group's snapshots
                (None, Some(Some(_)), None) => Deletion::ParentUnknown,
            };
        }
    }
}

/// The snapshots of one source subvolume, newest first.
#[derive(Clone, Debug, Serialize)]
pub struct Group {
    pub subvolume: String,
    pub snapshots: Vec<Snapshot>,
    /// Incremental chain, when the PC reports its snapshots.
    pub chain: Option<Chain>,
}

/// Where btrbk's next send of a subvolume starts from.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Chain {
    /// Newest complete snapshot present on both sides: the parent of the
    /// next incremental send.
    pub next_parent: Option<String>,
    /// The PC's snapshots not yet received here, newest first.
    pub pending: Vec<String>,
    /// No snapshot in common: the next send transfers everything.
    pub full_send: bool,
}

impl Group {
    /// Receive time of the newest complete snapshot (unix seconds).
    pub fn last_received(&self) -> Option<i64> {
        self.snapshots
            .iter()
            .filter(|s| s.problem.is_none())
            .filter_map(|s| s.info.as_ref().map(|i| if i.rtime > 0 { i.rtime } else { i.otime }))
            .max()
    }

    /// Space only these snapshots hold; None when quotas are off.
    pub fn exclusive_bytes(&self) -> Option<u64> {
        let usages: Vec<u64> = self.snapshots.iter().filter_map(|s| s.usage.map(|u| u.exclusive)).collect();
        (!usages.is_empty()).then(|| usages.iter().sum())
    }

    /// Real snapshots, without directories that only carry a snapshot name.
    pub fn subvolumes(&self) -> impl DoubleEndedIterator<Item = &Snapshot> {
        self.snapshots.iter().filter(|s| s.problem != Some(Problem::NotSubvolume))
    }

    pub fn count(&self, problem: Problem) -> usize {
        self.snapshots.iter().filter(|s| s.problem == Some(problem)).count()
    }
}

#[derive(Debug, Default)]
pub struct Scan {
    pub groups: Vec<Group>,
    /// Set when subvolume metadata could not be read (missing capability).
    pub metadata_error: Option<String>,
    /// The PC's report of its snapshots: None when it sends none.
    pub source: Option<Result<crate::source::Source, String>>,
}

/// Groups by subvolume, "@" first then alphabetically, newest first inside
/// each group. Names that are not snapshots are dropped.
pub fn group_names(names: impl IntoIterator<Item = String>) -> Vec<(String, Vec<String>)> {
    let mut groups: Vec<(String, Vec<String>)> = Vec::new();
    for name in names {
        let Some((prefix, _)) = split_snapshot_name(&name) else {
            continue;
        };
        match groups.iter_mut().find(|(existing, _)| existing == prefix) {
            Some((_, snapshots)) => snapshots.push(name),
            None => groups.push((prefix.to_string(), vec![name])),
        }
    }
    for (_, snapshots) in &mut groups {
        snapshots.sort_by(|a, b| b.cmp(a));
    }
    groups.sort_by(|(a, _), (b, _)| a.cmp(b));
    groups
}

/// Attaches metadata and problems to each snapshot.
///
/// The parent chain is not reconstructed: on a target, the parent UUID of a
/// received snapshot does not name any subvolume the target holds (seen on
/// DSM 7.4), and only the sending host knows which snapshot btrbk will use as
/// parent next.
pub fn analyse(
    groups: Vec<(String, Vec<String>)>,
    meta: &HashMap<String, Result<SubvolInfo, Problem>>,
    usage: &HashMap<u64, Usage>,
) -> Vec<Group> {
    groups
        .into_iter()
        .map(|(subvolume, names)| {
            let snapshots = names
                .into_iter()
                .map(|name| {
                    let timestamp = split_snapshot_name(&name).map(|(_, t)| t.to_string()).unwrap_or_default();
                    let (info, problem) = match meta.get(&name) {
                        Some(Ok(info)) => (Some(info.clone()), None),
                        Some(Err(problem)) => (None, Some(*problem)),
                        None => (None, None),
                    };
                    let problem = problem.or_else(|| {
                        let info = info.as_ref()?;
                        (info.received_uuid.is_none() || !info.readonly).then_some(Problem::Incomplete)
                    });
                    let usage = info.as_ref().and_then(|i| usage.get(&i.root_id).copied());
                    Snapshot { name, timestamp, problem, info, usage, on_source: None, created: None, deletion: Deletion::default() }
                })
                .collect();
            Group { subvolume, snapshots, chain: None }
        })
        .collect()
}

/// Marks the snapshots the PC still holds and works out each chain.
pub fn link_source(groups: &mut [Group], source: &crate::source::Source) {
    for group in groups {
        let mut received = std::collections::HashSet::new();
        for snap in &mut group.snapshots {
            let uuid = snap.info.as_ref().and_then(|i| i.received_uuid.clone());
            snap.on_source = Some(uuid.as_ref().is_some_and(|u| source.snapshots.contains_key(u)));
            snap.created = uuid.as_ref().and_then(|u| source.created.get(u).copied());
            received.extend(uuid);
        }
        let next_parent = group
            .snapshots
            .iter()
            .find(|s| s.problem.is_none() && s.on_source == Some(true))
            .map(|s| s.name.clone());
        let pending = source
            .snapshots
            .iter()
            .filter(|(uuid, name)| {
                !received.contains(*uuid)
                    && split_snapshot_name(name).is_some_and(|(prefix, _)| prefix == group.subvolume)
            })
            .map(|(_, name)| name.clone())
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .rev()
            .collect();
        group.chain = Some(Chain { full_send: next_parent.is_none(), next_parent, pending });
    }
}

/// Reads a btrbk target directory: names, then btrfs metadata for each.
pub fn scan(dir: &Path) -> std::io::Result<Scan> {
    let mut names = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir()
            && let Ok(name) = entry.file_name().into_string()
        {
            names.push(name);
        }
    }
    let groups = group_names(names);

    let mut scan = Scan::default();
    let mut meta = HashMap::new();
    let mut usage = HashMap::new();
    match btrfs::open_dir(dir).and_then(|fd| Ok((btrfs::list_subvolumes(&fd)?, btrfs::list_usage(&fd)))) {
        Ok((subvols, qgroups)) => {
            usage = qgroups;
            for name in groups.iter().flat_map(|(_, names)| names) {
                let path = dir.join(name);
                let is_subvol = fs::symlink_metadata(&path).is_ok_and(|m| m.ino() == SUBVOL_TOP_INO);
                let entry = if !is_subvol {
                    Err(Problem::NotSubvolume)
                } else {
                    match btrfs::open_dir(&path).and_then(|fd| btrfs::root_id(&fd)) {
                        Ok(id) => match subvols.get(&id) {
                            Some(info) => Ok(info.clone()),
                            None => continue,
                        },
                        Err(_) => continue,
                    }
                };
                meta.insert(name.clone(), entry);
            }
        }
        Err(e) => scan.metadata_error = Some(e.to_string()),
    }
    scan.groups = analyse(groups, &meta, &usage);
    scan.source = crate::source::load(dir);
    if let Some(Ok(source)) = &scan.source {
        link_source(&mut scan.groups, source);
    }
    mark_deletion(&mut scan.groups, crate::report::now());
    Ok(scan)
}

/// Deletes one snapshot of a target, after checking again, on fresh data,
/// that it may go. Deletions are serialised with a lock: two browser windows
/// cannot race each other.
pub fn delete(dir: &Path, name: &str, lock: &Path) -> Result<(), String> {
    if name.contains('/') || split_snapshot_name(name).is_none() {
        return Err(crate::lang::t!("non è il nome di uno snapshot", "not a snapshot name"));
    }
    let lock = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(lock)
        .map_err(|e| crate::lang::t!("impossibile aprire il lock: {}", "cannot open the lock: {}", e))?;
    // SAFETY: a valid descriptor; the lock goes with the file when dropped
    if unsafe { libc::flock(std::os::fd::AsRawFd::as_raw_fd(&lock), libc::LOCK_EX) } != 0 {
        return Err(crate::lang::t!("lock non ottenuto: {}", "cannot lock: {}", std::io::Error::last_os_error()));
    }

    let scan = scan(dir).map_err(|e| e.to_string())?;
    if let Some(e) = scan.metadata_error {
        return Err(crate::lang::t!("metadati btrfs non leggibili: {}", "cannot read btrfs metadata: {}", e));
    }
    let snapshot = scan
        .groups
        .iter()
        .flat_map(|g| &g.snapshots)
        .find(|s| s.name == name)
        .ok_or_else(|| crate::lang::t!("snapshot inesistente", "no such snapshot"))?;
    if !snapshot.deletion.allowed() {
        return Err(snapshot.deletion.refusal());
    }
    let parent = btrfs::open_dir(dir).map_err(|e| e.to_string())?;
    btrfs::destroy_subvolume(&parent, name).map_err(|e| e.to_string())
}

/// Looks for btrbk target directories: a share, or a directory directly
/// inside a share, holding at least one subvolume with a snapshot name.
pub fn discover(volumes_root: &Path) -> Vec<std::path::PathBuf> {
    let mut found = Vec::new();
    let Ok(volumes) = fs::read_dir(volumes_root) else {
        return found;
    };
    let mut volumes: Vec<_> = volumes
        .flatten()
        .filter(|e| e.file_name().to_str().is_some_and(|n| n.starts_with("volume")))
        .map(|e| e.path())
        .collect();
    volumes.sort();

    for volume in volumes {
        for share in sorted_subdirs(&volume, true) {
            if holds_snapshots(&share) {
                found.push(share);
                continue;
            }
            for dir in sorted_subdirs(&share, false) {
                if holds_snapshots(&dir) {
                    found.push(dir);
                }
            }
        }
    }
    found
}

fn sorted_subdirs(dir: &Path, skip_system: bool) -> Vec<std::path::PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut dirs: Vec<_> = entries
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
        .filter(|e| {
            let name = e.file_name();
            let name = name.to_string_lossy();
            // DSM's own directories: #recycle, #snapshot, @eaDir, @appstore...
            !(name.starts_with('#') || name == "@eaDir" || (skip_system && name.starts_with('@')))
        })
        .map(|e| e.path())
        .collect();
    dirs.sort();
    dirs
}

fn holds_snapshots(dir: &Path) -> bool {
    let Ok(entries) = fs::read_dir(dir) else {
        return false;
    };
    entries.flatten().any(|e| {
        e.file_name().to_str().is_some_and(|n| split_snapshot_name(n).is_some())
            && fs::symlink_metadata(e.path()).is_ok_and(|m| m.is_dir() && m.ino() == SUBVOL_TOP_INO)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_names_with_and_without_at() {
        assert_eq!(split_snapshot_name("@home.20260803T0000"), Some(("@home", "20260803T0000")));
        assert_eq!(split_snapshot_name("home.20250901T0800"), Some(("home", "20250901T0800")));
        assert_eq!(split_snapshot_name("my.data.20250901"), Some(("my.data", "20250901")));
        assert_eq!(split_snapshot_name("@.20260101T120000+0100_1"), Some(("@", "20260101T120000+0100_1")));
        assert_eq!(split_snapshot_name("scripts.d"), None);
        assert_eq!(split_snapshot_name(".20250901"), None);
        assert_eq!(split_snapshot_name("test-acl"), None);
    }

    #[test]
    fn groups_root_first_newest_first() {
        let names = ["@home.20260102", "@.20260101", "notes", "@.20260103", "@home.20260101"];
        let groups = group_names(names.iter().map(|s| s.to_string()));
        assert_eq!(groups[0], ("@".into(), vec!["@.20260103".into(), "@.20260101".into()]));
        assert_eq!(groups[1].0, "@home");
        assert_eq!(groups.len(), 2);
    }

    fn info(id: u64, received: bool, readonly: bool) -> SubvolInfo {
        SubvolInfo {
            root_id: id,
            readonly,
            uuid: Some(format!("u-{id}")),
            received_uuid: received.then(|| format!("r-{id}")),
            rtime: id as i64,
            ..SubvolInfo::default()
        }
    }

    #[test]
    fn deletes_only_what_the_chain_does_not_need() {
        let (d1, d2, d3, d4) = ("@.20261021", "@.20261022", "@.20261023", "@.20261024");
        let names = vec![d4.to_string(), d3.to_string(), d2.to_string(), d1.to_string()];
        let mut garbled = info(4, false, false);
        garbled.otime = 1000;
        let meta = HashMap::from([
            (d4.to_string(), Ok(garbled)),
            (d3.to_string(), Ok(info(3, true, true))),
            (d2.to_string(), Ok(info(2, true, true))),
            (d1.to_string(), Ok(info(1, true, true))),
        ]);
        let groups = || analyse(vec![("@".to_string(), names.clone())], &meta, &HashMap::new());
        let deletion = |g: &Vec<Group>| g[0].snapshots.iter().map(|s| s.deletion).collect::<Vec<_>>();

        // no report from the PC: only the old interrupted receive may go
        let mut g = groups();
        mark_deletion(&mut g, 1000 + RECEIVE_GRACE_SECS);
        assert_eq!(
            deletion(&g),
            [Deletion::AllowedIncomplete, Deletion::ParentUnknown, Deletion::ParentUnknown, Deletion::ParentUnknown]
        );
        // a young interrupted receive may still be running
        mark_deletion(&mut g, 1000 + 60);
        assert_eq!(g[0].snapshots[0].deletion, Deletion::Receiving);

        // the PC still holds d2 and d1: d2 is the parent, d1 may go, d3 stays
        let source = crate::source::Source {
            host: "PC".into(),
            generated: 1,
            snapshots: HashMap::from([("r-2".to_string(), d2.to_string()), ("r-1".to_string(), d1.to_string())]),
            created: HashMap::from([("r-2".to_string(), 1_793_020_335)]),
        };
        let mut g = groups();
        link_source(&mut g, &source);
        mark_deletion(&mut g, 1000 + RECEIVE_GRACE_SECS);
        assert_eq!(
            deletion(&g),
            [Deletion::AllowedIncomplete, Deletion::Newer, Deletion::Parent, Deletion::Allowed]
        );
        let created: Vec<_> = g[0].snapshots.iter().map(|s| s.created).collect();
        assert_eq!(created, [None, None, Some(1_793_020_335), None]);

        // nothing in common: left alone
        let lost = crate::source::Source { snapshots: HashMap::new(), ..source };
        let mut g = groups();
        link_source(&mut g, &lost);
        mark_deletion(&mut g, 1000 + RECEIVE_GRACE_SECS);
        assert_eq!(g[0].snapshots[1].deletion, Deletion::NoCommonParent);
    }

    #[test]
    fn finds_the_next_parent() {
        let (d1, d2, d3, d4) = ("@home.20261021", "@home.20261022", "@home.20261023", "@home.20261024");
        let groups = vec![("@home".to_string(), vec![d3.to_string(), d2.to_string(), d1.to_string()])];
        let meta = HashMap::from([
            (d3.to_string(), Ok(info(3, true, true))),
            (d2.to_string(), Ok(info(2, true, true))),
            (d1.to_string(), Ok(info(1, true, true))),
        ]);
        let mut groups = analyse(groups, &meta, &HashMap::new());
        // the PC pruned d3 locally and has a new d4 to send
        let source = crate::source::Source {
            host: "PC".into(),
            generated: 1,
            snapshots: HashMap::from([
                ("r-2".to_string(), d2.to_string()),
                ("r-1".to_string(), d1.to_string()),
                ("pc-4".to_string(), d4.to_string()),
                ("pc-x".to_string(), "@.20261024".to_string()),
            ]),
            ..Default::default()
        };
        link_source(&mut groups, &source);
        let chain = groups[0].chain.as_ref().unwrap();
        assert_eq!(chain.next_parent.as_deref(), Some(d2));
        assert_eq!(chain.pending, [d4]);
        assert!(!chain.full_send);
        let on: Vec<_> = groups[0].snapshots.iter().map(|s| s.on_source).collect();
        assert_eq!(on, [Some(false), Some(true), Some(true)]);

        // nothing in common any more: the next send is a full one
        let lost = crate::source::Source { snapshots: HashMap::from([("pc-4".into(), d4.into())]), ..source };
        link_source(&mut groups, &lost);
        assert!(groups[0].chain.as_ref().unwrap().full_send);
    }

    #[test]
    fn flags_incomplete_receives() {
        let groups = vec![("@".to_string(), vec!["@.4".into(), "@.3".into(), "@.2".into(), "@.1".into()])];
        let meta = HashMap::from([
            ("@.4".to_string(), Ok(info(4, false, true))),
            ("@.3".to_string(), Ok(info(3, true, false))),
            ("@.2".to_string(), Ok(info(2, true, true))),
            ("@.1".to_string(), Err(Problem::NotSubvolume)),
        ]);
        let usage = HashMap::from([(2, Usage { referenced: 10, exclusive: 2 })]);
        let group = &analyse(groups, &meta, &usage)[0];
        let s = &group.snapshots;
        assert_eq!(s[0].problem, Some(Problem::Incomplete));
        assert_eq!(s[1].problem, Some(Problem::Incomplete));
        assert_eq!(s[2].problem, None);
        assert_eq!(s[3].problem, Some(Problem::NotSubvolume));
        assert_eq!(s[2].usage.unwrap().exclusive, 2);
        // incomplete receives do not count as the last backup
        assert_eq!(group.last_received(), Some(2));
    }
}
