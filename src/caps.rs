//! The capabilities the package needs, and how they get there.
//!
//! DSM 7 refuses the `capabilities` field of conf/privilege in third-party
//! packages ("invalid package privilege content", error 319), so the
//! package installs unprivileged and `btrbk-nas setup`, run once as root,
//! writes the file capability itself. Reinstalling or upgrading the package
//! replaces the binary and drops it: setup has to run again.
//!
//! The capabilities are granted at exec, all three. The CGI and the daemon
//! put them aside at once (`suspend`) and take back only what they need
//! (`limit`): the CGI after the login check and for its route only, the
//! daemon without CAP_DAC_OVERRIDE, which only deleting needs.

use std::ffi::CString;
use std::io;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

/// TREE_SEARCH and INO_LOOKUP on the root tree.
pub const SYS_ADMIN: u64 = 1 << 21;
/// Reading every file of a snapshot, whoever owns it.
pub const DAC_READ_SEARCH: u64 = 1 << 2;
/// Deleting a snapshot removes an entry from the target directory: the
/// kernel wants write permission on it (btrfs_may_delete).
pub const DAC_OVERRIDE: u64 = 1 << 1;

/// Reading the backups: btrfs metadata and every file.
pub const READ: u64 = SYS_ADMIN | DAC_READ_SEARCH;
pub const ALL: u64 = READ | DAC_OVERRIDE;

const VFS_CAP_REVISION_2: u32 = 0x0200_0000;
const VFS_CAP_FLAGS_EFFECTIVE: u32 = 0x1;
const LINUX_CAPABILITY_VERSION_3: u32 = 0x2008_0522;

/// Whether the process started with all three, before `suspend`.
static GRANTED: AtomicBool = AtomicBool::new(false);

#[repr(C)]
struct CapHeader {
    version: u32,
    pid: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct CapData {
    effective: u32,
    permitted: u32,
    inheritable: u32,
}

fn get() -> io::Result<(u64, u64)> {
    let mut header = CapHeader { version: LINUX_CAPABILITY_VERSION_3, pid: 0 };
    let mut data = [CapData::default(); 2];
    // SAFETY: a version 3 header and the two data words it asks for
    if unsafe { libc::syscall(libc::SYS_capget, &mut header, data.as_mut_ptr()) } != 0 {
        return Err(io::Error::last_os_error());
    }
    let join = |low: u32, high: u32| u64::from(low) | u64::from(high) << 32;
    Ok((join(data[0].effective, data[1].effective), join(data[0].permitted, data[1].permitted)))
}

fn set(effective: u64, permitted: u64) -> io::Result<()> {
    let mut header = CapHeader { version: LINUX_CAPABILITY_VERSION_3, pid: 0 };
    let data = [
        CapData { effective: effective as u32, permitted: permitted as u32, inheritable: 0 },
        CapData { effective: (effective >> 32) as u32, permitted: (permitted >> 32) as u32, inheritable: 0 },
    ];
    // SAFETY: a version 3 header and two data words
    if unsafe { libc::syscall(libc::SYS_capset, &mut header, data.as_ptr()) } != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

/// Clears the effective set and keeps the permitted one, to be taken back
/// with `limit`. Remembers whether setup had run.
/// As root (the development server) nothing changes: root's files are its
/// own by uid, not by capability.
pub fn suspend() -> io::Result<()> {
    let (_, permitted) = get()?;
    GRANTED.store(permitted & ALL == ALL, Ordering::Relaxed);
    if is_root() {
        return Ok(());
    }
    set(0, permitted)
}

/// Keeps only `wanted` (of what is permitted), effective from now on; the
/// rest is gone for good.
pub fn limit(wanted: u64) -> io::Result<()> {
    if is_root() {
        return Ok(());
    }
    let (_, permitted) = get()?;
    let kept = permitted & wanted;
    set(kept, kept)
}

fn is_root() -> bool {
    // SAFETY: geteuid cannot fail
    (unsafe { libc::geteuid() }) == 0
}

/// Whether `setup` had run when this process started.
pub fn granted() -> bool {
    GRANTED.load(Ordering::Relaxed)
}

/// The security.capability xattr value: revision 2, effective, permitted
/// set = our three capabilities, nothing inheritable.
pub fn xattr_value() -> [u8; 20] {
    let permitted = ALL;
    let words = [
        VFS_CAP_REVISION_2 | VFS_CAP_FLAGS_EFFECTIVE,
        permitted as u32,
        0,
        (permitted >> 32) as u32,
        0,
    ];
    let mut value = [0u8; 20];
    for (chunk, word) in value.as_chunks_mut::<4>().0.iter_mut().zip(words) {
        chunk.copy_from_slice(&word.to_le_bytes());
    }
    value
}

/// Whether `binary` carries the file capabilities written by `install`.
pub fn on_file(binary: &Path) -> bool {
    let Ok(path) = CString::new(binary.as_os_str().as_bytes()) else {
        return false;
    };
    let mut value = [0u8; 20];
    // SAFETY: valid C strings and a buffer of the stated length
    let len = unsafe {
        libc::getxattr(path.as_ptr(), c"security.capability".as_ptr(), value.as_mut_ptr().cast(), value.len())
    };
    len == 20 && value == xattr_value()
}

/// Gives `binary` the file capabilities. Needs root.
///
/// The package user must not be able to change what root runs here next
/// time: the binary and the directories up to the package's target become
/// root's first (chown would drop the capability, so it comes last). The
/// binary stays executable by the package's group only.
pub fn install(binary: &Path) -> io::Result<()> {
    if let Some((target, bin_dir)) = package_dirs(binary) {
        let group = std::fs::metadata(binary)?.gid();
        for dir in [target, bin_dir] {
            std::os::unix::fs::lchown(dir, Some(0), Some(0))?;
            std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o755))?;
        }
        std::os::unix::fs::lchown(binary, Some(0), Some(group))?;
        std::fs::set_permissions(binary, std::fs::Permissions::from_mode(0o750))?;
    }

    let path = CString::new(binary.as_os_str().as_bytes()).map_err(io::Error::other)?;
    let name = c"security.capability";
    let value = xattr_value();
    // SAFETY: valid C strings and a buffer of the stated length
    let rc = unsafe { libc::setxattr(path.as_ptr(), name.as_ptr(), value.as_ptr().cast(), value.len(), 0) };
    if rc < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

/// `<target>/bin/btrbk-nas` of the installed package, as (target, bin);
/// None for a development build, whose directories stay as they are.
fn package_dirs(binary: &Path) -> Option<(&Path, &Path)> {
    let bin_dir = binary.parent()?;
    let target = bin_dir.parent()?;
    (bin_dir.file_name()? == "bin" && target.file_name()? == crate::config::PACKAGE).then_some((target, bin_dir))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xattr_matches_setcap() {
        // `setcap cap_sys_admin,cap_dac_read_search,cap_dac_override+ep` writes exactly this
        assert_eq!(
            xattr_value(),
            [0x01, 0, 0, 0x02, 0x06, 0, 0x20, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
        );
    }

    #[test]
    fn hardens_only_the_installed_package() {
        let installed = Path::new("/volume2/@appstore/btrbk-nas/bin/btrbk-nas");
        assert_eq!(
            package_dirs(installed),
            Some((Path::new("/volume2/@appstore/btrbk-nas"), Path::new("/volume2/@appstore/btrbk-nas/bin")))
        );
        assert_eq!(package_dirs(Path::new("/home/me/btrbk-nas/target/debug/btrbk-nas")), None);
    }
}
