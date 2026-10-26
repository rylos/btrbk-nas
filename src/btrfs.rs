//! Read-only btrfs metadata through ioctls.
//!
//! DSM runs a 4.4 kernel with btrfs-progs 4.0: the unprivileged
//! GET_SUBVOL_INFO ioctl (4.18) is missing, and parsing the output of such an
//! old `btrfs` binary would be fragile. Subvolume details come from
//! TREE_SEARCH on the root tree instead, which needs CAP_SYS_ADMIN.

use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::os::fd::AsRawFd;
use std::path::Path;

use serde::Serialize;

const IOC_MAGIC: u64 = 0x94;

const fn iowr(nr: u64, size: u64) -> u64 {
    (3 << 30) | (size << 16) | (IOC_MAGIC << 8) | nr
}

const IOC_TREE_SEARCH: u64 = iowr(17, 4096);
const IOC_INO_LOOKUP: u64 = iowr(18, 4096);

const ROOT_TREE_OBJECTID: u64 = 1;
const QUOTA_TREE_OBJECTID: u64 = 8;
const FIRST_FREE_OBJECTID: u64 = 256;
const LAST_FREE_OBJECTID: u64 = u64::MAX - 255;
const ROOT_ITEM_KEY: u32 = 132;
const QGROUP_INFO_KEY: u32 = 242;
const ROOT_SUBVOL_RDONLY: u64 = 1;

/// The inode number of every subvolume's top directory.
pub const SUBVOL_TOP_INO: u64 = 256;

#[repr(C)]
struct SearchKey {
    tree_id: u64,
    min_objectid: u64,
    max_objectid: u64,
    min_offset: u64,
    max_offset: u64,
    min_transid: u64,
    max_transid: u64,
    min_type: u32,
    max_type: u32,
    nr_items: u32,
    unused: u32,
    unused1: u64,
    unused2: u64,
    unused3: u64,
    unused4: u64,
}

const SEARCH_BUF: usize = 4096 - std::mem::size_of::<SearchKey>();
const SEARCH_HEADER: usize = 32;

#[repr(C)]
struct SearchArgs {
    key: SearchKey,
    buf: [u8; SEARCH_BUF],
}

#[repr(C)]
struct InoLookupArgs {
    treeid: u64,
    objectid: u64,
    name: [u8; 4080],
}

/// What the root tree knows about one subvolume.
#[derive(Clone, Debug, Default, Serialize)]
pub struct SubvolInfo {
    pub root_id: u64,
    pub generation: u64,
    pub readonly: bool,
    pub uuid: Option<String>,
    pub parent_uuid: Option<String>,
    pub received_uuid: Option<String>,
    /// Creation time of the subvolume on this filesystem: when the receive
    /// started.
    pub otime: i64,
    /// When `btrfs receive` finished; 0 for a receive that never completed.
    pub rtime: i64,
}

/// Space accounting from qgroups, when quotas are enabled on the volume.
#[derive(Clone, Copy, Debug, Default, Serialize)]
pub struct Usage {
    /// Bytes the snapshot references, shared or not.
    pub referenced: u64,
    /// Bytes only this snapshot holds: what deleting it would free.
    pub exclusive: u64,
}

fn le64(data: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(data[at..at + 8].try_into().unwrap())
}

fn uuid_at(data: &[u8], at: usize) -> Option<String> {
    let bytes = &data[at..at + 16];
    if bytes.iter().all(|&b| b == 0) {
        return None;
    }
    let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    Some(format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    ))
}

// Offsets inside struct btrfs_root_item (packed, little endian).
const RI_GENERATION: usize = 160;
const RI_FLAGS: usize = 208;
const RI_GENERATION_V2: usize = 239;
const RI_UUID: usize = 247;
const RI_PARENT_UUID: usize = 263;
const RI_RECEIVED_UUID: usize = 279;
const RI_OTIME: usize = 339;
const RI_RTIME: usize = 363;
const RI_V2_LEN: usize = 375;

/// Decodes a ROOT_ITEM. Items written by kernels older than 3.5 lack the
/// uuid and time fields; generation_v2 tells whether they are valid.
pub fn parse_root_item(root_id: u64, data: &[u8]) -> Option<SubvolInfo> {
    if data.len() < RI_FLAGS + 8 {
        return None;
    }
    let generation = le64(data, RI_GENERATION);
    let mut info = SubvolInfo {
        root_id,
        generation,
        readonly: le64(data, RI_FLAGS) & ROOT_SUBVOL_RDONLY != 0,
        ..SubvolInfo::default()
    };
    if data.len() >= RI_V2_LEN && le64(data, RI_GENERATION_V2) == generation {
        info.uuid = uuid_at(data, RI_UUID);
        info.parent_uuid = uuid_at(data, RI_PARENT_UUID);
        info.received_uuid = uuid_at(data, RI_RECEIVED_UUID);
        info.otime = le64(data, RI_OTIME) as i64;
        info.rtime = le64(data, RI_RTIME) as i64;
    }
    Some(info)
}

/// Runs TREE_SEARCH over every item of type `item_type` with objectid in
/// `objectids`, calling `found(objectid, offset, data)` for each.
fn tree_search(
    fd: &File,
    tree_id: u64,
    objectids: (u64, u64),
    item_type: u32,
    mut found: impl FnMut(u64, u64, &[u8]),
) -> io::Result<()> {
    // SAFETY: SearchArgs is plain old data, all zeroes is a valid value
    let mut args: Box<SearchArgs> = Box::new(unsafe { std::mem::zeroed() });
    args.key.tree_id = tree_id;
    args.key.min_objectid = objectids.0;
    args.key.max_objectid = objectids.1;
    args.key.max_offset = u64::MAX;
    args.key.max_transid = u64::MAX;
    args.key.min_type = item_type;
    args.key.max_type = item_type;

    loop {
        args.key.nr_items = 4096;
        // SAFETY: the kernel writes at most size_of::<SearchArgs>() bytes
        let rc = unsafe { libc::ioctl(fd.as_raw_fd(), IOC_TREE_SEARCH as _, &mut *args) };
        if rc < 0 {
            return Err(io::Error::last_os_error());
        }
        let count = args.key.nr_items;
        if count == 0 {
            return Ok(());
        }

        let (mut pos, mut last) = (0usize, None);
        for _ in 0..count {
            if pos + SEARCH_HEADER > SEARCH_BUF {
                break;
            }
            let header = &args.buf[pos..pos + SEARCH_HEADER];
            let objectid = le64(header, 8);
            let offset = le64(header, 16);
            let kind = u32::from_le_bytes(header[24..28].try_into().unwrap());
            let len = u32::from_le_bytes(header[28..32].try_into().unwrap()) as usize;
            pos += SEARCH_HEADER;
            if pos + len > SEARCH_BUF {
                break;
            }
            // the range is compared as a whole (objectid, type, offset) key,
            // so items of other types between the bounds come back too
            if kind == item_type {
                found(objectid, offset, &args.buf[pos..pos + len]);
            }
            pos += len;
            last = Some((objectid, kind, offset));
        }

        let Some(last) = last else {
            return Err(io::Error::other("tree search returned an unreadable item"));
        };
        match next_key(last) {
            Some((objectid, kind, offset)) if objectid <= objectids.1 => {
                args.key.min_objectid = objectid;
                args.key.min_type = kind;
                args.key.min_offset = offset;
            }
            _ => return Ok(()),
        }
    }
}

/// The smallest key after `key`, None past the last possible one.
fn next_key((objectid, kind, offset): (u64, u32, u64)) -> Option<(u64, u32, u64)> {
    if offset < u64::MAX {
        Some((objectid, kind, offset + 1))
    } else if kind < 255 {
        Some((objectid, kind + 1, 0))
    } else {
        objectid.checked_add(1).map(|objectid| (objectid, 0, 0))
    }
}

/// Every subvolume of the filesystem holding `fd`, by root id.
pub fn list_subvolumes(fd: &File) -> io::Result<HashMap<u64, SubvolInfo>> {
    let mut subvols = HashMap::new();
    tree_search(
        fd,
        ROOT_TREE_OBJECTID,
        (FIRST_FREE_OBJECTID, LAST_FREE_OBJECTID),
        ROOT_ITEM_KEY,
        |objectid, _, data| {
            if let Some(info) = parse_root_item(objectid, data) {
                subvols.insert(objectid, info);
            }
        },
    )?;
    Ok(subvols)
}

/// Level-0 qgroup usage by subvolume root id. Empty when quotas are off.
pub fn list_usage(fd: &File) -> HashMap<u64, Usage> {
    let mut usage = HashMap::new();
    // the quota tree does not exist when quotas were never enabled: ENOENT
    let _ = tree_search(fd, QUOTA_TREE_OBJECTID, (0, 0), QGROUP_INFO_KEY, |_, qgroupid, data| {
        if qgroupid >> 48 == 0 && data.len() >= 40 {
            usage.insert(
                qgroupid,
                Usage {
                    referenced: le64(data, 8),
                    exclusive: le64(data, 24),
                },
            );
        }
    });
    usage
}

/// Root id of the subvolume `dir` belongs to.
pub fn root_id(dir: &File) -> io::Result<u64> {
    // SAFETY: plain old data
    let mut args: Box<InoLookupArgs> = Box::new(unsafe { std::mem::zeroed() });
    args.objectid = FIRST_FREE_OBJECTID;
    // SAFETY: the kernel writes at most size_of::<InoLookupArgs>() bytes
    let rc = unsafe { libc::ioctl(dir.as_raw_fd(), IOC_INO_LOOKUP as _, &mut *args) };
    if rc < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(args.treeid)
}

/// Opens a directory for ioctls without following a final symlink.
pub fn open_dir(path: &Path) -> io::Result<File> {
    use std::os::unix::fs::OpenOptionsExt;
    std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW)
        .open(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root_item(generation_v2: u64) -> Vec<u8> {
        let mut data = vec![0u8; 439];
        data[RI_GENERATION..RI_GENERATION + 8].copy_from_slice(&42u64.to_le_bytes());
        data[RI_FLAGS] = 1;
        data[RI_GENERATION_V2..RI_GENERATION_V2 + 8].copy_from_slice(&generation_v2.to_le_bytes());
        for (i, b) in data[RI_UUID..RI_UUID + 16].iter_mut().enumerate() {
            *b = i as u8 + 1;
        }
        data[RI_RECEIVED_UUID] = 0xab;
        data[RI_RTIME..RI_RTIME + 8].copy_from_slice(&1_790_000_000u64.to_le_bytes());
        data
    }

    #[test]
    fn decodes_a_received_snapshot() {
        let info = parse_root_item(300, &root_item(42)).unwrap();
        assert_eq!(info.root_id, 300);
        assert!(info.readonly);
        assert_eq!(info.uuid.as_deref(), Some("01020304-0506-0708-090a-0b0c0d0e0f10"));
        assert_eq!(info.parent_uuid, None);
        assert_eq!(info.received_uuid.as_deref(), Some("ab000000-0000-0000-0000-000000000000"));
        assert_eq!(info.rtime, 1_790_000_000);
    }

    #[test]
    fn ignores_v2_fields_of_an_old_item() {
        let info = parse_root_item(300, &root_item(7)).unwrap();
        assert_eq!(info.uuid, None);
        assert_eq!(info.rtime, 0);
    }

    #[test]
    fn next_key_carries_over() {
        assert_eq!(next_key((300, 144, 5)), Some((300, 144, 6)));
        assert_eq!(next_key((300, 144, u64::MAX)), Some((300, 145, 0)));
        assert_eq!(next_key((300, 255, u64::MAX)), Some((301, 0, 0)));
        assert_eq!(next_key((u64::MAX, 255, u64::MAX)), None);
    }

    #[test]
    fn ioctl_numbers_match_the_kernel() {
        assert_eq!(IOC_TREE_SEARCH, 0xd000_9411);
        assert_eq!(IOC_INO_LOOKUP, 0xd000_9412);
        assert_eq!(std::mem::size_of::<SearchArgs>(), 4096);
        assert_eq!(std::mem::size_of::<InoLookupArgs>(), 4096);
    }
}
