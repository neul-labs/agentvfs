//! Attribute conversion utilities for FUSE.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use fuser::FileAttr;

use crate::fs::FileEntry;

/// Default TTL for cached attributes (1 second).
pub const TTL: Duration = Duration::from_secs(1);

/// Default block size for the filesystem.
pub const BLOCK_SIZE: u32 = 4096;

/// Convert a FileEntry to fuser::FileAttr.
pub fn entry_to_attr(entry: &FileEntry) -> FileAttr {
    let kind = if entry.file_type.is_dir() {
        fuser::FileType::Directory
    } else {
        fuser::FileType::RegularFile
    };

    // Convert chrono DateTime to SystemTime
    let mtime = datetime_to_system_time(&entry.modified_at);
    let ctime = datetime_to_system_time(&entry.created_at);

    // Get current user's uid/gid
    let uid = unsafe { libc::getuid() };
    let gid = unsafe { libc::getgid() };

    // Permissions: directories get 755, files get 644
    let perm = if entry.file_type.is_dir() {
        0o755
    } else {
        0o644
    };

    FileAttr {
        ino: entry.id as u64,
        size: entry.size,
        blocks: (entry.size + 511) / 512,
        atime: mtime, // Use mtime for atime (we don't track access time)
        mtime,
        ctime,
        crtime: ctime, // Creation time
        kind,
        perm,
        nlink: 1,
        uid,
        gid,
        rdev: 0,
        blksize: BLOCK_SIZE,
        flags: 0,
    }
}

/// Create FileAttr for a special entry (root, etc.).
pub fn root_attr(id: u64) -> FileAttr {
    let now = SystemTime::now();
    let uid = unsafe { libc::getuid() };
    let gid = unsafe { libc::getgid() };

    FileAttr {
        ino: id,
        size: 0,
        blocks: 0,
        atime: now,
        mtime: now,
        ctime: now,
        crtime: now,
        kind: fuser::FileType::Directory,
        perm: 0o755,
        nlink: 2,
        uid,
        gid,
        rdev: 0,
        blksize: BLOCK_SIZE,
        flags: 0,
    }
}

/// Convert chrono DateTime<Utc> to SystemTime.
fn datetime_to_system_time(dt: &chrono::DateTime<chrono::Utc>) -> SystemTime {
    let secs = dt.timestamp();
    let nanos = dt.timestamp_subsec_nanos();

    if secs >= 0 {
        UNIX_EPOCH + Duration::new(secs as u64, nanos)
    } else {
        // Handle dates before epoch (unlikely but possible)
        UNIX_EPOCH
    }
}
