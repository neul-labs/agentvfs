//! FUSE filesystem implementation for vfs.

use std::collections::HashMap;
use std::ffi::OsStr;
use std::sync::Mutex;

use fuser::{
    Filesystem, ReplyAttr, ReplyData, ReplyDirectory, ReplyEmpty, ReplyEntry, ReplyOpen,
    ReplyStatfs, ReplyWrite, Request,
};

use crate::error::VfsError;
use crate::fs::FileSystem;

use super::attr::{entry_to_attr, root_attr, BLOCK_SIZE, TTL};

/// Root inode number (FUSE convention).
const ROOT_INODE: u64 = 1;

/// Information about an open file.
struct OpenFile {
    /// Path to the file (for operations that need it).
    path: String,
    /// Whether opened for writing.
    write: bool,
    /// Buffered file content for write handles.
    buffer: Option<Vec<u8>>,
    /// Whether the buffered content differs from storage.
    dirty: bool,
}

/// VFS FUSE filesystem implementation.
pub struct VfsFilesystem {
    /// The underlying VFS filesystem.
    fs: FileSystem,
    /// Cache of inode to path mappings.
    inode_to_path: Mutex<HashMap<u64, String>>,
    /// Next file handle to assign.
    next_fh: Mutex<u64>,
    /// Open file handles.
    open_files: Mutex<HashMap<u64, OpenFile>>,
    /// Whether mounted read-only.
    readonly: bool,
}

impl VfsFilesystem {
    /// Create a new VFS FUSE filesystem.
    pub fn new(fs: FileSystem, readonly: bool) -> Self {
        let mut inode_to_path = HashMap::new();
        // Root is always inode 1 and path "/"
        inode_to_path.insert(ROOT_INODE, "/".to_string());

        Self {
            fs,
            inode_to_path: Mutex::new(inode_to_path),
            next_fh: Mutex::new(1),
            open_files: Mutex::new(HashMap::new()),
            readonly,
        }
    }

    /// Get path for an inode, or None if not cached.
    fn get_path(&self, ino: u64) -> Option<String> {
        self.inode_to_path.lock().unwrap().get(&ino).cloned()
    }

    /// Cache a path for an inode.
    fn cache_path(&self, ino: u64, path: String) {
        self.inode_to_path.lock().unwrap().insert(ino, path);
    }

    /// Remove a cached path.
    fn uncache_path(&self, ino: u64) {
        self.inode_to_path.lock().unwrap().remove(&ino);
    }

    /// Get the next file handle.
    fn next_file_handle(&self) -> u64 {
        let mut next = self.next_fh.lock().unwrap();
        let fh = *next;
        *next += 1;
        fh
    }

    /// Convert VfsError to libc errno.
    fn error_to_errno(e: &VfsError) -> i32 {
        match e {
            VfsError::NotFound(_) => libc::ENOENT,
            VfsError::AlreadyExists(_) => libc::EEXIST,
            VfsError::NotADirectory(_) => libc::ENOTDIR,
            VfsError::NotAFile(_) => libc::EISDIR,
            VfsError::NotEmpty(_) => libc::ENOTEMPTY,
            VfsError::InvalidPath(_) => libc::EINVAL,
            VfsError::InvalidInput(_) => libc::EINVAL,
            VfsError::QuotaExceeded(_) => libc::ENOSPC,
            VfsError::Io(_) => libc::EIO,
            _ => libc::EIO,
        }
    }

    /// Build a child path from parent path and name.
    fn child_path(parent: &str, name: &str) -> String {
        if parent == "/" {
            format!("/{}", name)
        } else {
            format!("{}/{}", parent, name)
        }
    }

    fn apply_write(buffer: &mut Vec<u8>, offset: usize, data: &[u8]) {
        if offset + data.len() > buffer.len() {
            buffer.resize(offset + data.len(), 0);
        }
        buffer[offset..offset + data.len()].copy_from_slice(data);
    }

    fn persist_open_file(&self, fh: u64) -> Result<(), VfsError> {
        let pending = {
            let open_files = self.open_files.lock().unwrap();
            let Some(open_file) = open_files.get(&fh) else {
                return Ok(());
            };

            if !open_file.write || !open_file.dirty {
                return Ok(());
            }

            let buffer = open_file.buffer.clone().unwrap_or_default();
            let path = open_file.path.clone();
            Some((path, buffer))
        };

        if let Some((path, buffer)) = pending {
            self.fs.write_file(&path, &buffer)?;
            if let Some(open_file) = self.open_files.lock().unwrap().get_mut(&fh) {
                open_file.dirty = false;
            }
        }

        Ok(())
    }
}

impl Filesystem for VfsFilesystem {
    /// Look up a directory entry by name.
    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        let name = match name.to_str() {
            Some(n) => n,
            None => {
                reply.error(libc::EINVAL);
                return;
            }
        };

        // Get parent path
        let parent_path = match self.get_path(parent) {
            Some(p) => p,
            None => {
                reply.error(libc::ENOENT);
                return;
            }
        };

        // Build child path
        let child_path = Self::child_path(&parent_path, name);

        // Look up the entry
        match self.fs.get_entry(&child_path) {
            Ok(entry) => {
                let attr = entry_to_attr(&entry);
                self.cache_path(entry.id as u64, child_path);
                reply.entry(&TTL, &attr, 0);
            }
            Err(e) => {
                reply.error(Self::error_to_errno(&e));
            }
        }
    }

    /// Get file attributes.
    fn getattr(&mut self, _req: &Request, ino: u64, reply: ReplyAttr) {
        if ino == ROOT_INODE {
            // Handle root specially - get it from the filesystem
            match self.fs.get_entry("/") {
                Ok(entry) => {
                    let attr = entry_to_attr(&entry);
                    reply.attr(&TTL, &attr);
                }
                Err(_) => {
                    // Fallback to synthetic root attr
                    reply.attr(&TTL, &root_attr(ROOT_INODE));
                }
            }
            return;
        }

        // Get path for inode
        let path = match self.get_path(ino) {
            Some(p) => p,
            None => {
                reply.error(libc::ENOENT);
                return;
            }
        };

        // Get entry
        match self.fs.get_entry(&path) {
            Ok(entry) => {
                let attr = entry_to_attr(&entry);
                reply.attr(&TTL, &attr);
            }
            Err(e) => {
                reply.error(Self::error_to_errno(&e));
            }
        }
    }

    /// Read directory entries.
    fn readdir(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        mut reply: ReplyDirectory,
    ) {
        // Get path for inode
        let path = match self.get_path(ino) {
            Some(p) => p,
            None => {
                reply.error(libc::ENOENT);
                return;
            }
        };

        // Get parent inode for ".."
        let parent_ino = if ino == ROOT_INODE {
            ROOT_INODE
        } else {
            // Try to get parent from the entry
            match self.fs.get_entry(&path) {
                Ok(entry) => entry.parent_id.map(|id| id as u64).unwrap_or(ROOT_INODE),
                Err(_) => ROOT_INODE,
            }
        };

        // Build entries list
        let mut entries = vec![
            (ino, fuser::FileType::Directory, ".".to_string()),
            (parent_ino, fuser::FileType::Directory, "..".to_string()),
        ];

        // List directory contents
        match self.fs.list_dir(&path) {
            Ok(dir_entries) => {
                for entry in dir_entries {
                    let child_path = Self::child_path(&path, &entry.name);

                    // Get the file ID for this entry
                    if let Ok(file_entry) = self.fs.get_entry(&child_path) {
                        let kind = if entry.file_type.is_dir() {
                            fuser::FileType::Directory
                        } else {
                            fuser::FileType::RegularFile
                        };

                        self.cache_path(file_entry.id as u64, child_path);
                        entries.push((file_entry.id as u64, kind, entry.name));
                    }
                }
            }
            Err(e) => {
                reply.error(Self::error_to_errno(&e));
                return;
            }
        }

        // Return entries starting from offset
        for (i, (ino, kind, name)) in entries.iter().enumerate().skip(offset as usize) {
            if reply.add(*ino, (i + 1) as i64, *kind, name) {
                break;
            }
        }

        reply.ok();
    }

    /// Open a file.
    fn open(&mut self, _req: &Request, ino: u64, flags: i32, reply: ReplyOpen) {
        // Get path
        let path = match self.get_path(ino) {
            Some(p) => p,
            None => {
                reply.error(libc::ENOENT);
                return;
            }
        };

        // Check if file exists
        match self.fs.get_entry(&path) {
            Ok(entry) => {
                if entry.is_dir() {
                    reply.error(libc::EISDIR);
                    return;
                }

                // Check readonly mode
                let write_flags = libc::O_WRONLY | libc::O_RDWR | libc::O_APPEND | libc::O_TRUNC;
                let is_write = (flags & write_flags) != 0;

                if self.readonly && is_write {
                    reply.error(libc::EROFS);
                    return;
                }

                let truncate = (flags & libc::O_TRUNC) != 0;
                let buffer = if is_write {
                    if truncate {
                        Some(Vec::new())
                    } else {
                        Some(match self.fs.read_file(&path) {
                            Ok(data) => data,
                            Err(VfsError::NotFound(_)) => Vec::new(),
                            Err(e) => {
                                reply.error(Self::error_to_errno(&e));
                                return;
                            }
                        })
                    }
                } else {
                    None
                };

                let fh = self.next_file_handle();
                self.open_files.lock().unwrap().insert(
                    fh,
                    OpenFile {
                        path,
                        write: is_write,
                        buffer,
                        dirty: false,
                    },
                );

                reply.opened(fh, 0);
            }
            Err(e) => {
                reply.error(Self::error_to_errno(&e));
            }
        }
    }

    /// Read file contents.
    fn read(
        &mut self,
        _req: &Request,
        ino: u64,
        fh: u64,
        offset: i64,
        size: u32,
        _flags: i32,
        _lock_owner: Option<u64>,
        reply: ReplyData,
    ) {
        // Get path from file handle or inode
        let buffered = {
            let open_files = self.open_files.lock().unwrap();
            if let Some(of) = open_files.get(&fh) {
                of.buffer.clone()
            } else {
                None
            }
        };

        if let Some(data) = buffered {
            let offset = offset as usize;
            let size = size as usize;

            if offset >= data.len() {
                reply.data(&[]);
            } else {
                let end = std::cmp::min(offset + size, data.len());
                reply.data(&data[offset..end]);
            }
            return;
        }

        let path = match self.get_path(ino) {
            Some(p) => p,
            None => {
                reply.error(libc::ENOENT);
                return;
            }
        };

        match self.fs.read_file(&path) {
            Ok(data) => {
                let offset = offset as usize;
                let size = size as usize;

                if offset >= data.len() {
                    reply.data(&[]);
                } else {
                    let end = std::cmp::min(offset + size, data.len());
                    reply.data(&data[offset..end]);
                }
            }
            Err(e) => {
                reply.error(Self::error_to_errno(&e));
            }
        }
    }

    /// Write to a file.
    fn write(
        &mut self,
        _req: &Request,
        _ino: u64,
        fh: u64,
        offset: i64,
        data: &[u8],
        _write_flags: u32,
        _flags: i32,
        _lock_owner: Option<u64>,
        reply: ReplyWrite,
    ) {
        if self.readonly {
            reply.error(libc::EROFS);
            return;
        }

        let mut open_files = self.open_files.lock().unwrap();
        let Some(open_file) = open_files.get_mut(&fh) else {
            reply.error(libc::ENOENT);
            return;
        };

        if !open_file.write {
            reply.error(libc::EBADF);
            return;
        }

        let Some(buffer) = open_file.buffer.as_mut() else {
            reply.error(libc::EIO);
            return;
        };

        Self::apply_write(buffer, offset as usize, data);
        open_file.dirty = true;
        reply.written(data.len() as u32);
    }

    /// Release (close) a file.
    fn release(
        &mut self,
        _req: &Request,
        _ino: u64,
        fh: u64,
        _flags: i32,
        _lock_owner: Option<u64>,
        _flush: bool,
        reply: ReplyEmpty,
    ) {
        if let Err(e) = self.persist_open_file(fh) {
            reply.error(Self::error_to_errno(&e));
            return;
        }
        self.open_files.lock().unwrap().remove(&fh);
        reply.ok();
    }

    /// Create a file.
    fn create(
        &mut self,
        _req: &Request,
        parent: u64,
        name: &OsStr,
        _mode: u32,
        _umask: u32,
        _flags: i32,
        reply: fuser::ReplyCreate,
    ) {
        if self.readonly {
            reply.error(libc::EROFS);
            return;
        }

        let name = match name.to_str() {
            Some(n) => n,
            None => {
                reply.error(libc::EINVAL);
                return;
            }
        };

        // Get parent path
        let parent_path = match self.get_path(parent) {
            Some(p) => p,
            None => {
                reply.error(libc::ENOENT);
                return;
            }
        };

        let child_path = Self::child_path(&parent_path, name);

        // Create empty file
        match self.fs.write_file(&child_path, &[]) {
            Ok(()) => {
                // Get the created entry
                match self.fs.get_entry(&child_path) {
                    Ok(entry) => {
                        let attr = entry_to_attr(&entry);
                        self.cache_path(entry.id as u64, child_path.clone());

                        let fh = self.next_file_handle();
                        self.open_files.lock().unwrap().insert(
                            fh,
                            OpenFile {
                                path: child_path,
                                write: true,
                                buffer: Some(Vec::new()),
                                dirty: false,
                            },
                        );

                        reply.created(&TTL, &attr, 0, fh, 0);
                    }
                    Err(e) => {
                        reply.error(Self::error_to_errno(&e));
                    }
                }
            }
            Err(e) => {
                reply.error(Self::error_to_errno(&e));
            }
        }
    }

    /// Remove a file.
    fn unlink(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEmpty) {
        if self.readonly {
            reply.error(libc::EROFS);
            return;
        }

        let name = match name.to_str() {
            Some(n) => n,
            None => {
                reply.error(libc::EINVAL);
                return;
            }
        };

        let parent_path = match self.get_path(parent) {
            Some(p) => p,
            None => {
                reply.error(libc::ENOENT);
                return;
            }
        };

        let child_path = Self::child_path(&parent_path, name);

        // Get entry to find its inode before deletion
        let ino = self.fs.get_entry(&child_path).ok().map(|e| e.id as u64);

        match self.fs.remove(&child_path, false) {
            Ok(()) => {
                if let Some(ino) = ino {
                    self.uncache_path(ino);
                }
                reply.ok();
            }
            Err(e) => {
                reply.error(Self::error_to_errno(&e));
            }
        }
    }

    /// Create a directory.
    fn mkdir(
        &mut self,
        _req: &Request,
        parent: u64,
        name: &OsStr,
        _mode: u32,
        _umask: u32,
        reply: ReplyEntry,
    ) {
        if self.readonly {
            reply.error(libc::EROFS);
            return;
        }

        let name = match name.to_str() {
            Some(n) => n,
            None => {
                reply.error(libc::EINVAL);
                return;
            }
        };

        let parent_path = match self.get_path(parent) {
            Some(p) => p,
            None => {
                reply.error(libc::ENOENT);
                return;
            }
        };

        let child_path = Self::child_path(&parent_path, name);

        match self.fs.create_dir(&child_path) {
            Ok(()) => match self.fs.get_entry(&child_path) {
                Ok(entry) => {
                    let attr = entry_to_attr(&entry);
                    self.cache_path(entry.id as u64, child_path);
                    reply.entry(&TTL, &attr, 0);
                }
                Err(e) => {
                    reply.error(Self::error_to_errno(&e));
                }
            },
            Err(e) => {
                reply.error(Self::error_to_errno(&e));
            }
        }
    }

    /// Remove a directory.
    fn rmdir(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEmpty) {
        if self.readonly {
            reply.error(libc::EROFS);
            return;
        }

        let name = match name.to_str() {
            Some(n) => n,
            None => {
                reply.error(libc::EINVAL);
                return;
            }
        };

        let parent_path = match self.get_path(parent) {
            Some(p) => p,
            None => {
                reply.error(libc::ENOENT);
                return;
            }
        };

        let child_path = Self::child_path(&parent_path, name);

        // Get inode before deletion
        let ino = self.fs.get_entry(&child_path).ok().map(|e| e.id as u64);

        match self.fs.remove(&child_path, false) {
            Ok(()) => {
                if let Some(ino) = ino {
                    self.uncache_path(ino);
                }
                reply.ok();
            }
            Err(e) => {
                reply.error(Self::error_to_errno(&e));
            }
        }
    }

    /// Rename a file or directory.
    fn rename(
        &mut self,
        _req: &Request,
        parent: u64,
        name: &OsStr,
        newparent: u64,
        newname: &OsStr,
        _flags: u32,
        reply: ReplyEmpty,
    ) {
        if self.readonly {
            reply.error(libc::EROFS);
            return;
        }

        let name = match name.to_str() {
            Some(n) => n,
            None => {
                reply.error(libc::EINVAL);
                return;
            }
        };

        let newname = match newname.to_str() {
            Some(n) => n,
            None => {
                reply.error(libc::EINVAL);
                return;
            }
        };

        let parent_path = match self.get_path(parent) {
            Some(p) => p,
            None => {
                reply.error(libc::ENOENT);
                return;
            }
        };

        let newparent_path = match self.get_path(newparent) {
            Some(p) => p,
            None => {
                reply.error(libc::ENOENT);
                return;
            }
        };

        let old_path = Self::child_path(&parent_path, name);
        let new_path = Self::child_path(&newparent_path, newname);

        // Get old inode
        let old_ino = self.fs.get_entry(&old_path).ok().map(|e| e.id as u64);

        match self.fs.move_entry(&old_path, &new_path) {
            Ok(()) => {
                // Update path cache
                if let Some(ino) = old_ino {
                    self.uncache_path(ino);
                    self.cache_path(ino, new_path);
                }
                reply.ok();
            }
            Err(e) => {
                reply.error(Self::error_to_errno(&e));
            }
        }
    }

    /// Get filesystem statistics.
    fn statfs(&mut self, _req: &Request, _ino: u64, reply: ReplyStatfs) {
        // Get vault stats
        let stats = self.fs.backend().get_vault_stats();

        match stats {
            Ok(stats) => {
                // Report reasonable values
                let blocks = stats.total_size_bytes / BLOCK_SIZE as u64 + 1;
                let files = stats.files + stats.directories;

                reply.statfs(
                    blocks * 10,   // Total blocks (give 10x headroom)
                    blocks * 9,    // Free blocks
                    blocks * 9,    // Available blocks
                    files + 10000, // Total inodes
                    10000,         // Free inodes
                    BLOCK_SIZE,    // Block size
                    255,           // Max name length
                    BLOCK_SIZE,    // Fragment size
                );
            }
            Err(_) => {
                // Return default stats on error
                reply.statfs(
                    1000000, // blocks
                    900000,  // bfree
                    900000,  // bavail
                    100000,  // files
                    90000,   // ffree
                    BLOCK_SIZE, 255, BLOCK_SIZE,
                );
            }
        }
    }

    /// Flush file data.
    fn flush(&mut self, _req: &Request, _ino: u64, fh: u64, _lock_owner: u64, reply: ReplyEmpty) {
        match self.persist_open_file(fh) {
            Ok(()) => reply.ok(),
            Err(e) => reply.error(Self::error_to_errno(&e)),
        }
    }

    /// Sync file data.
    fn fsync(&mut self, _req: &Request, _ino: u64, fh: u64, _datasync: bool, reply: ReplyEmpty) {
        match self.persist_open_file(fh) {
            Ok(()) => reply.ok(),
            Err(e) => reply.error(Self::error_to_errno(&e)),
        }
    }

    /// Open a directory.
    fn opendir(&mut self, _req: &Request, ino: u64, _flags: i32, reply: ReplyOpen) {
        // Verify directory exists
        let path = match self.get_path(ino) {
            Some(p) => p,
            None => {
                reply.error(libc::ENOENT);
                return;
            }
        };

        match self.fs.get_entry(&path) {
            Ok(entry) => {
                if !entry.is_dir() {
                    reply.error(libc::ENOTDIR);
                    return;
                }
                // Return a dummy file handle
                reply.opened(0, 0);
            }
            Err(e) => {
                reply.error(Self::error_to_errno(&e));
            }
        }
    }

    /// Release (close) a directory.
    fn releasedir(&mut self, _req: &Request, _ino: u64, _fh: u64, _flags: i32, reply: ReplyEmpty) {
        reply.ok();
    }

    /// Set file attributes (mainly for truncate).
    fn setattr(
        &mut self,
        _req: &Request,
        ino: u64,
        _mode: Option<u32>,
        _uid: Option<u32>,
        _gid: Option<u32>,
        size: Option<u64>,
        _atime: Option<fuser::TimeOrNow>,
        _mtime: Option<fuser::TimeOrNow>,
        _ctime: Option<std::time::SystemTime>,
        fh: Option<u64>,
        _crtime: Option<std::time::SystemTime>,
        _chgtime: Option<std::time::SystemTime>,
        _bkuptime: Option<std::time::SystemTime>,
        _flags: Option<u32>,
        reply: ReplyAttr,
    ) {
        if self.readonly {
            reply.error(libc::EROFS);
            return;
        }

        let path = match self.get_path(ino) {
            Some(p) => p,
            None => {
                reply.error(libc::ENOENT);
                return;
            }
        };

        // Handle truncate
        if let Some(new_size) = size {
            if let Some(fh) = fh {
                let mut open_files = self.open_files.lock().unwrap();
                if let Some(open_file) = open_files.get_mut(&fh) {
                    if let Some(buffer) = open_file.buffer.as_mut() {
                        buffer.resize(new_size as usize, 0);
                        open_file.dirty = true;
                    } else {
                        reply.error(libc::EBADF);
                        return;
                    }
                } else {
                    reply.error(libc::ENOENT);
                    return;
                }
            } else {
                let content = match self.fs.read_file(&path) {
                    Ok(c) => c,
                    Err(VfsError::NotFound(_)) => Vec::new(),
                    Err(e) => {
                        reply.error(Self::error_to_errno(&e));
                        return;
                    }
                };

                let new_content = if new_size == 0 {
                    Vec::new()
                } else if new_size as usize <= content.len() {
                    content[..new_size as usize].to_vec()
                } else {
                    let mut new = content;
                    new.resize(new_size as usize, 0);
                    new
                };

                if let Err(e) = self.fs.write_file(&path, &new_content) {
                    reply.error(Self::error_to_errno(&e));
                    return;
                }
            }
        }

        // Return updated attributes
        match self.fs.get_entry(&path) {
            Ok(entry) => {
                let attr = entry_to_attr(&entry);
                reply.attr(&TTL, &attr);
            }
            Err(e) => {
                reply.error(Self::error_to_errno(&e));
            }
        }
    }
}
