//! What the sending PC still holds, as it reports it.
//!
//! btrbk's next incremental parent is the newest snapshot present on both
//! sides, and the NAS alone cannot know which snapshots the PC kept. After
//! every run the PC writes the list of its snapshots into the target
//! directory (extras/pc/btrbk-nas-push-uuids); a PC snapshot's UUID is the
//! received UUID of its copy on the NAS.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::Serialize;

pub const FILE: &str = ".btrbk-nas-source";
const HEADER: &str = "# btrbk-nas source v1";

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Source {
    pub host: String,
    /// When the PC wrote the list (unix seconds).
    pub generated: i64,
    /// Snapshot UUID on the PC -> snapshot name.
    #[serde(skip)]
    pub snapshots: HashMap<String, String>,
}

pub fn parse(text: &str) -> Result<Source, String> {
    let mut lines = text.lines();
    if lines.next() != Some(HEADER) {
        return Err(format!("not a {FILE} file"));
    }
    let mut source = Source::default();
    for line in lines.filter(|l| !l.trim().is_empty()) {
        let mut fields = line.splitn(3, ' ');
        match (fields.next(), fields.next(), fields.next()) {
            (Some("host"), Some(host), None) => source.host = host.to_string(),
            (Some("generated"), Some(epoch), None) => {
                source.generated = epoch.parse().map_err(|_| format!("bad generated time: {epoch}"))?;
            }
            (Some("snapshot"), Some(uuid), Some(name)) => {
                source.snapshots.insert(uuid.to_string(), name.to_string());
            }
            _ => return Err(format!("unexpected line: {line}")),
        }
    }
    Ok(source)
}

/// The PC's report in `dir`: None when the PC does not send one.
pub fn load(dir: &Path) -> Option<Result<Source, String>> {
    match fs::read_to_string(dir.join(FILE)) {
        Ok(text) => Some(parse(&text)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => Some(Err(e.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "# btrbk-nas source v1
host desktop
generated 1793017000
snapshot 11111111-0000-0000-0000-000000000001 @.20261026T0847
snapshot 22222222-0000-0000-0000-000000000002 @home.20261026T0847
snapshot 33333333-0000-0000-0000-000000000003 @home.20261026T1312
";

    #[test]
    fn reads_the_pc_report() {
        let source = parse(SAMPLE).unwrap();
        assert_eq!(source.host, "desktop");
        assert_eq!(source.generated, 1_793_017_000);
        assert_eq!(source.snapshots["22222222-0000-0000-0000-000000000002"], "@home.20261026T0847");
        assert_eq!(source.snapshots.len(), 3);
    }

    #[test]
    fn refuses_other_files() {
        assert!(parse("hello").is_err());
        assert!(parse("# btrbk-nas source v1\ngenerated soon\n").is_err());
    }
}
