//! Files inside a snapshot: listing, single downloads, whole directories as
//! a tar stream.
//!
//! Nothing here goes by path. The snapshot is opened once and checked to be
//! a read-only subvolume; everything below it is reached one name at a time
//! from the directory already open, without following symlinks and without
//! leaving the snapshot's subvolume. Swapping entries of the target, or of a
//! snapshot whose owner made it writable again, cannot lead anywhere else.

use std::ffi::{CString, OsStr, OsString};
use std::fs::{self, File, Metadata};
use std::io::{self, Read, Write};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{FileTypeExt, MetadataExt};
use std::path::{Path, PathBuf};
use std::rc::Rc;

use serde::Serialize;

use crate::cgi::ApiError;
use crate::lang::t;
use crate::snapshot::split_snapshot_name;

/// Listing cap: a directory with more entries is cut, and says so.
const MAX_ENTRIES: usize = 10_000;

/// Something inside a snapshot, held open.
#[derive(Debug)]
pub struct Item {
    /// Its name: the last part of the path, or the snapshot's own.
    pub name: String,
    /// An O_PATH handle: it names this very inode, whatever happens to the
    /// path afterwards.
    handle: File,
    pub meta: Metadata,
}

impl Item {
    /// Opens the item for reading, if it is a regular file.
    pub fn open_file(&self) -> io::Result<File> {
        if !self.meta.is_file() {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "not a file"));
        }
        File::open(fd_path(&self.handle))
    }

    pub fn handle(&self) -> &File {
        &self.handle
    }
}

/// The handle's own inode, reopened through procfs.
fn fd_path(file: &File) -> PathBuf {
    PathBuf::from(format!("/proc/self/fd/{}", file.as_raw_fd()))
}

/// `name` in `dir` as an O_PATH handle; a symlink is the link itself.
fn handle_in(dir: &File, name: &OsStr) -> io::Result<File> {
    let name = CString::new(name.as_bytes()).map_err(io::Error::other)?;
    // SAFETY: a valid directory descriptor and C string
    let fd = unsafe {
        libc::openat(dir.as_raw_fd(), name.as_ptr(), libc::O_PATH | libc::O_NOFOLLOW | libc::O_CLOEXEC)
    };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: a descriptor we just opened and own
    Ok(unsafe { File::from_raw_fd(fd) })
}

/// Resolves `rel` inside snapshot `name` of `target`, refusing anything that
/// could point outside it.
pub fn resolve(target: &Path, name: &str, rel: &str) -> Result<Item, ApiError> {
    if name.contains('/') || split_snapshot_name(name).is_none() {
        return Err(ApiError::bad_request(t!("non è il nome di uno snapshot", "not a snapshot name")));
    }
    let snapshot = crate::btrfs::open_dir(&target.join(name)).map_err(|_| ApiError::not_found(t!("snapshot inesistente", "no such snapshot")))?;
    if !crate::btrfs::is_readonly_subvolume(&snapshot).unwrap_or(false) {
        // an interrupted receive is still writable: nothing in it holds still
        return Err(ApiError::new(
            409,
            t!(
                "non è uno snapshot in sola lettura: una ricezione interrotta, o non è un subvolume",
                "not a read-only snapshot: an interrupted receive, or not a subvolume"
            ),
        ));
    }
    resolve_in(snapshot, name, rel)
}

/// Walks `rel` from the open `snapshot`, one name at a time.
fn resolve_in(snapshot: File, name: &str, rel: &str) -> Result<Item, ApiError> {
    let root = snapshot.metadata()?;
    let mut item = Item { name: name.to_string(), handle: snapshot, meta: root.clone() };
    for part in rel.split('/').filter(|p| !p.is_empty()) {
        if part == "." || part == ".." {
            return Err(ApiError::bad_request(t!("percorso non valido", "invalid path")));
        }
        if !item.meta.is_dir() {
            return Err(ApiError::bad_request(t!(
                "il percorso passa per qualcosa che non è una cartella",
                "path goes through something that is not a directory"
            )));
        }
        let handle =
            handle_in(&item.handle, OsStr::new(part)).map_err(|_| ApiError::not_found(t!("file inesistente in questo snapshot", "no such file in this snapshot")))?;
        let meta = handle.metadata()?;
        if meta.dev() != root.dev() {
            // a nested subvolume or a mount: not part of this snapshot
            return Err(ApiError::bad_request(t!("il percorso esce dallo snapshot", "path leaves the snapshot")));
        }
        item = Item { name: part.to_string(), handle, meta };
    }
    Ok(item)
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
pub fn list(dir: &File) -> Result<Listing, ApiError> {
    if !dir.metadata()?.is_dir() {
        return Err(ApiError::bad_request(t!("non è una cartella", "not a directory")));
    }
    let mut entries = Vec::new();
    let mut truncated = false;
    // entries are read relative to the open directory, never followed
    for entry in fs::read_dir(fd_path(dir))? {
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

/// Writes a POSIX ustar archive of the open directory `root`, entry names
/// under `base` so the archive unpacks into one directory. Names too long
/// for ustar get a GNU long-name record, which GNU tar, bsdtar and 7-Zip
/// read. Nested subvolumes and mounts are left out.
pub fn write_tar(root: &File, base: &str, out: &mut impl Write) -> io::Result<()> {
    let dev = root.metadata()?.dev();
    let root = Rc::new(root.try_clone()?);
    tar_header(out, &format!("{base}/"), &root.metadata()?, b'5', 0, "")?;
    // (directory, entry in it, name in the archive)
    let mut stack: Vec<(Rc<File>, OsString, PathBuf)> = Vec::new();
    push_children(&mut stack, &root, Path::new(base))?;
    while let Some((dir, child, name)) = stack.pop() {
        let Ok(handle) = handle_in(&dir, &child) else { continue };
        let Ok(meta) = handle.metadata() else { continue };
        if meta.dev() != dev {
            continue;
        }
        let t = meta.file_type();
        let text = name.to_string_lossy().into_owned();
        if t.is_dir() {
            tar_header(out, &format!("{text}/"), &meta, b'5', 0, "")?;
            let handle = Rc::new(handle);
            push_children(&mut stack, &handle, &name)?;
        } else if t.is_file() {
            let mut file = File::open(fd_path(&handle))?;
            tar_header(out, &text, &meta, b'0', meta.len(), "")?;
            // never write more or less than the header promised
            let copied = io::copy(&mut (&mut file).take(meta.len()), out)?;
            write_zeros(out, meta.len() - copied)?;
            write_zeros(out, padding(meta.len()))?;
        } else if t.is_symlink() {
            let link = fs::read_link(fd_path(&dir).join(&child))?.to_string_lossy().into_owned();
            tar_header(out, &text, &meta, b'2', 0, &link)?;
        } else if t.is_fifo() || t.is_char_device() || t.is_block_device() || t.is_socket() {
            // device nodes and sockets carry no data worth restoring
            continue;
        }
    }
    // end of archive: two zero blocks
    write_zeros(out, 1024)
}

fn push_children(stack: &mut Vec<(Rc<File>, OsString, PathBuf)>, dir: &Rc<File>, name: &Path) -> io::Result<()> {
    let mut children: Vec<_> = fs::read_dir(fd_path(dir))?.flatten().map(|e| e.file_name()).collect();
    // reverse, so popping visits them in order
    children.sort_by(|a, b| b.cmp(a));
    stack.extend(children.into_iter().map(|c| (Rc::clone(dir), c.clone(), name.join(c))));
    Ok(())
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
        let name = "@home.20261026T0847";
        let snap = target.join(name);
        fs::create_dir_all(snap.join("alice/docs")).unwrap();
        fs::write(snap.join("alice/docs/a.txt"), "a").unwrap();
        std::os::unix::fs::symlink("/etc", snap.join("alice/etc")).unwrap();
        std::os::unix::fs::symlink("/etc/hostname", snap.join("alice/host")).unwrap();
        let at = |rel: &str| resolve_in(File::open(&snap).unwrap(), name, rel);

        let item = at("alice/docs/a.txt").unwrap();
        assert_eq!(item.name, "a.txt");
        assert_eq!(fs::read_to_string(fd_path(&item.open_file().unwrap())).unwrap(), "a");
        assert_eq!(at("").unwrap().name, name);
        assert_eq!(at("alice/../..").err().unwrap().status, 400);
        // the link itself can be resolved (and listed), never walked through
        assert!(at("alice/etc").unwrap().meta.is_symlink());
        assert_eq!(at("alice/etc/passwd").err().unwrap().status, 400);
        assert!(at("alice/host").unwrap().open_file().is_err());
        assert_eq!(at("alice/nothing").err().unwrap().status, 404);
        // the snapshot has to be a read-only subvolume, which a plain directory is not
        assert_eq!(resolve(&target, name, "").err().unwrap().status, 409);
        assert_eq!(resolve(&target, "../x.20261026", "").err().unwrap().status, 400);
        assert_eq!(resolve(&target, "notes", "").err().unwrap().status, 400);
        assert_eq!(resolve(&target, "@.20261026T0847", "").err().unwrap().status, 404);
        fs::remove_dir_all(target).unwrap();
    }

    #[test]
    fn lists_directories_first() {
        let dir = scratch("list");
        fs::write(dir.join("b.txt"), "bb").unwrap();
        fs::create_dir(dir.join("Zeta")).unwrap();
        fs::write(dir.join("a.txt"), "a").unwrap();
        let names: Vec<_> = list(&File::open(&dir).unwrap()).unwrap().entries.into_iter().map(|e| e.name).collect();
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
        write_tar(&File::open(&root).unwrap(), "docs", &mut File::create(&archive).unwrap()).unwrap();
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
