//! The two capabilities the package needs, and how they get there.
//!
//! DSM 7 refuses the `capabilities` field of conf/privilege in third-party
//! packages ("invalid package privilege content", error 319), so the
//! package installs unprivileged and `btrbk-nas setup`, run once as root,
//! writes the file capability itself. Reinstalling or upgrading the package
//! replaces the binary and drops it: setup has to run again.

use std::ffi::CString;
use std::io;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

/// TREE_SEARCH and INO_LOOKUP on the root tree.
const CAP_SYS_ADMIN: u32 = 21;
/// Reading every file of a snapshot, whoever owns it.
const CAP_DAC_READ_SEARCH: u32 = 2;

const VFS_CAP_REVISION_2: u32 = 0x0200_0000;
const VFS_CAP_FLAGS_EFFECTIVE: u32 = 0x1;

fn mask() -> u64 {
    (1 << CAP_SYS_ADMIN) | (1 << CAP_DAC_READ_SEARCH)
}

/// Whether this process holds both capabilities right now.
pub fn effective() -> bool {
    let status = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    status
        .lines()
        .find_map(|line| line.strip_prefix("CapEff:"))
        .and_then(|hex| u64::from_str_radix(hex.trim(), 16).ok())
        .is_some_and(|eff| eff & mask() == mask())
}

/// The security.capability xattr value: revision 2, effective, permitted
/// set = our two capabilities, nothing inheritable.
pub fn xattr_value() -> [u8; 20] {
    let permitted = mask();
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
pub fn install(binary: &Path) -> io::Result<()> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xattr_matches_setcap() {
        // `setcap cap_sys_admin,cap_dac_read_search+ep` writes exactly this
        assert_eq!(
            xattr_value(),
            [0x01, 0, 0, 0x02, 0x04, 0, 0x20, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
        );
    }
}
