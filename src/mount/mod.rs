//! FUSE mount support for vfs.
//!
//! This module provides the ability to mount a VFS vault as a real directory
//! using FUSE (Filesystem in Userspace).

mod attr;
mod filesystem;

pub use filesystem::VfsFilesystem;
