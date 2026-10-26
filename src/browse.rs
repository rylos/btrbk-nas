//! Files inside a snapshot: listing, single downloads, whole directories as
//! a tar stream.
//!
//! Received snapshots are read-only, so a path checked once stays what it
//! was: walking it with lstat is enough to never leave the snapshot through a
//! symlink or "..".

use std::fs::{self, File, Metadata};
use std::io::{self, Read, Write};
use std::os::unix::fs::{FileTypeExt, MetadataExt};
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::cgi::ApiError;
use crate::snapshot::split_snapshot_name;

/// Listing cap: a directory with more entries is cut, and says so.
const MAX_ENTRIES: usize = 10_000;

/// Resolves `rel` inside snapshot `name` of `target`, refusing anything that
/// could point outside it.
pub fn resolve(target: &Path, name: &str, rel: &str) -> Result<PathBuf, ApiError> {
    if name.contains('/') || split_snapshot_name(name).is_none() {
        return Err(ApiError::bad_request("not a snapshot name"));
    }
    let mut path = target.join(name);
    let meta = fs::symlink_metadata(&path).map_err(|_| ApiError::not_found("no such snapshot"))?;
    if !meta.is_dir() {
        return Err(ApiError::not_found("no such snapshot"));
    }
    for part in rel.split('/').filter(|p| !p.is_empty()) {
        if part == "." || part == ".." {
            return Err(ApiError::bad_request("invalid path"));
        }
        let parent_is_dir = fs::symlink_metadata(&path).is_ok_and(|m| m.is_dir());
        if !parent_is_dir {
            return Err(ApiError::bad_request("path goes through something that is not a directory"));
        }
        path.push(part);
    }
    fs::symlink_metadata(&path).map_err(|_| ApiError::not_found("no such file in this snapshot"))?;
    Ok(path)
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Dir,
    File,
    Symlink,
    Other,
}

#[derive(Debug, Serialize)]
pub struct Entry {
    pub name: String,
    pub kind: Kind,
    pub size: u64,
    pub mtime: i64,
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
    /// Where a symlink points; it is shown, never followed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Listing {
    pub entries: Vec<Entry>,
    pub truncated: bool,
}

fn kind(meta: &Metadata) -> Kind {
    let t = meta.file_type();
    if t.is_dir() {
        Kind::Dir
    } else if t.is_file() {
        Kind::File
    } else if t.is_symlink() {
        Kind::Symlink
    } else {
        Kind::Other
    }
}

/// Directories first, then files, each alphabetically.
pub fn list(dir: &Path) -> Result<Listing, ApiError> {
    if !fs::symlink_metadata(dir)?.is_dir() {
        return Err(ApiError::bad_request("not a directory"));
    }
    let mut entries = Vec::new();
    let mut truncated = false;
    for entry in fs::read_dir(dir)? {
        if entries.len() == MAX_ENTRIES {
            truncated = true;
            break;
        }
        let entry = entry?;
        let Ok(meta) = entry.metadata() else { continue };
        let kind = kind(&meta);
        let link = (kind == Kind::Symlink)
            .then(|| fs::read_link(entry.path()).ok().map(|l| l.to_string_lossy().into_owned()))
            .flatten();
        entries.push(Entry {
            name: entry.file_name().to_string_lossy().into_owned(),
            kind,
            size: meta.len(),
            mtime: meta.mtime(),
            mode: meta.mode() & 0o7777,
            uid: meta.uid(),
            gid: meta.gid(),
            link,
        });
    }
    entries.sort_by(|a, b| {
        (a.kind != Kind::Dir)
            .cmp(&(b.kind != Kind::Dir))
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Ok(Listing { entries, truncated })
}

/// Writes a POSIX ustar archive of `root`, entry names relative to its
/// parent so the archive unpacks into one directory. Names too long for
/// ustar get a GNU long-name record, which GNU tar, bsdtar and 7-Zip read.
pub fn write_tar(root: &Path, out: &mut impl Write) -> io::Result<()> {
    let base = root.file_name().map(PathBuf::from).unwrap_or_else(|| PathBuf::from("snapshot"));
    let mut stack = vec![(root.to_path_buf(), base)];
    while let Some((path, name)) = stack.pop() {
        let Ok(meta) = fs::symlink_metadata(&path) else { continue };
        let t = meta.file_type();
        let mut name = name.to_string_lossy().into_owned();
        if t.is_dir() {
            name.push('/');
            tar_header(out, &name, &meta, b'5', 0, "")?;
            let mut children: Vec<_> = fs::read_dir(&path)?.flatten().map(|e| e.file_name()).collect();
            // reverse, so popping visits them in order
            children.sort_by(|a, b| b.cmp(a));
            let parent = PathBuf::from(name.trim_end_matches('/'));
            stack.extend(children.into_iter().map(|c| (path.join(&c), parent.join(c))));
        } else if t.is_file() {
            let mut file = File::open(&path)?;
            tar_header(out, &name, &meta, b'0', meta.len(), "")?;
            // never write more or less than the header promised
            let copied = io::copy(&mut (&mut file).take(meta.len()), out)?;
            write_zeros(out, meta.len() - copied)?;
            write_zeros(out, padding(meta.len()))?;
        } else if t.is_symlink() {
            let link = fs::read_link(&path)?.to_string_lossy().into_owned();
            tar_header(out, &name, &meta, b'2', 0, &link)?;
        } else if t.is_fifo() || t.is_char_device() || t.is_block_device() || t.is_socket() {
            // device nodes and sockets carry no data worth restoring
            continue;
        }
    }
    // end of archive: two zero blocks
    write_zeros(out, 1024)
}

fn padding(size: u64) -> u64 {
    (512 - size % 512) % 512
}

fn write_zeros(out: &mut impl Write, mut count: u64) -> io::Result<()> {
    let zeros = [0u8; 512];
    while count > 0 {
        let n = count.min(512) as usize;
        out.write_all(&zeros[..n])?;
        count -= n as u64;
    }
    Ok(())
}

fn octal(field: &mut [u8], value: u64) {
    let width = field.len() - 1;
    let text = format!("{value:0width$o}");
    let text = text.as_bytes();
    // a value too large for the field keeps its low digits: only mtime of
    // files from the far future could hit this
    field[..width].copy_from_slice(&text[text.len() - width..]);
}

fn tar_header(out: &mut impl Write, name: &str, meta: &Metadata, typeflag: u8, size: u64, link: &str) -> io::Result<()> {
    if name.len() > 100 || link.len() > 100 {
        // GNU long name ('L') / long link ('K') records precede the entry
        for (flag, value) in [(b'L', name), (b'K', link)] {
            if value.len() > 100 {
                let mut record = value.as_bytes().to_vec();
                record.push(0);
                raw_header(out, "././@LongLink", 0o644, 0, 0, 0, flag, record.len() as u64, "")?;
                out.write_all(&record)?;
                write_zeros(out, padding(record.len() as u64))?;
            }
        }
    }
    let mtime = u64::try_from(meta.mtime()).unwrap_or(0);
    raw_header(out, name, meta.mode() & 0o7777, meta.uid(), meta.gid(), mtime, typeflag, size, link)
}

#[allow(clippy::too_many_arguments)]
fn raw_header(
    out: &mut impl Write,
    name: &str,
    mode: u32,
    uid: u32,
    gid: u32,
    mtime: u64,
    typeflag: u8,
    size: u64,
    link: &str,
) -> io::Result<()> {
    let mut h = [0u8; 512];
    let name = name.as_bytes();
    h[..name.len().min(100)].copy_from_slice(&name[..name.len().min(100)]);
    octal(&mut h[100..108], u64::from(mode));
    octal(&mut h[108..116], u64::from(uid));
    octal(&mut h[116..124], u64::from(gid));
    if size >= 0o77777777777 {
        // base-256 for files of 8 GiB and more (GNU extension)
        h[124] = 0x80;
        h[124 + 4..136].copy_from_slice(&size.to_be_bytes());
    } else {
        octal(&mut h[124..136], size);
    }
    octal(&mut h[136..148], mtime);
    h[156] = typeflag;
    let link = link.as_bytes();
    h[157..157 + link.len().min(100)].copy_from_slice(&link[..link.len().min(100)]);
    h[257..263].copy_from_slice(b"ustar\0");
    h[263..265].copy_from_slice(b"00");
    // checksum is computed with its own field set to spaces
    h[148..156].copy_from_slice(b"        ");
    let sum: u32 = h.iter().map(|&b| u32::from(b)).sum();
    let text = format!("{sum:06o}\0 ");
    h[148..156].copy_from_slice(text.as_bytes());
    out.write_all(&h)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("btrbk-nas-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn resolve_stays_inside_the_snapshot() {
        let target = scratch("resolve");
        let snap = target.join("@home.20261026T0847");
        fs::create_dir_all(snap.join("alice/docs")).unwrap();
        fs::write(snap.join("alice/docs/a.txt"), "a").unwrap();
        std::os::unix::fs::symlink("/etc", snap.join("alice/etc")).unwrap();

        assert!(resolve(&target, "@home.20261026T0847", "alice/docs/a.txt").is_ok());
        assert!(resolve(&target, "@home.20261026T0847", "").is_ok());
        assert_eq!(resolve(&target, "@home.20261026T0847", "alice/../..").unwrap_err().status, 400);
        // the link itself can be resolved (and listed), never walked through
        assert!(resolve(&target, "@home.20261026T0847", "alice/etc").is_ok());
        assert_eq!(resolve(&target, "@home.20261026T0847", "alice/etc/passwd").unwrap_err().status, 400);
        assert_eq!(resolve(&target, "../x.20261026", "").unwrap_err().status, 400);
        assert_eq!(resolve(&target, "notes", "").unwrap_err().status, 400);
        assert_eq!(resolve(&target, "@.20261026T0847", "").unwrap_err().status, 404);
        fs::remove_dir_all(target).unwrap();
    }

    #[test]
    fn lists_directories_first() {
        let dir = scratch("list");
        fs::write(dir.join("b.txt"), "bb").unwrap();
        fs::create_dir(dir.join("Zeta")).unwrap();
        fs::write(dir.join("a.txt"), "a").unwrap();
        let names: Vec<_> = list(&dir).unwrap().entries.into_iter().map(|e| e.name).collect();
        assert_eq!(names, ["Zeta", "a.txt", "b.txt"]);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn tar_unpacks_with_gnu_tar() {
        let dir = scratch("tar");
        let root = dir.join("docs");
        let long = "x".repeat(120);
        fs::create_dir_all(root.join("sub")).unwrap();
        fs::write(root.join("sub/hello.txt"), "hello\n").unwrap();
        fs::write(root.join(&long), vec![7u8; 1000]).unwrap();
        fs::write(root.join("empty"), "").unwrap();
        std::os::unix::fs::symlink("sub/hello.txt", root.join("link")).unwrap();

        let archive = dir.join("out.tar");
        write_tar(&root, &mut File::create(&archive).unwrap()).unwrap();
        let unpack = dir.join("unpack");
        fs::create_dir(&unpack).unwrap();
        let status = Command::new("tar").arg("-xf").arg(&archive).arg("-C").arg(&unpack).status().unwrap();
        assert!(status.success());
        assert_eq!(fs::read_to_string(unpack.join("docs/sub/hello.txt")).unwrap(), "hello\n");
        assert_eq!(fs::read(unpack.join("docs").join(&long)).unwrap(), vec![7u8; 1000]);
        assert_eq!(fs::read_link(unpack.join("docs/link")).unwrap(), Path::new("sub/hello.txt"));
        assert_eq!(fs::metadata(unpack.join("docs/empty")).unwrap().len(), 0);
        fs::remove_dir_all(dir).unwrap();
    }
}
