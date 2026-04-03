//! agentvfs - Virtual filesystem CLI backed by embedded databases.
//!
//! This library provides a virtual filesystem abstraction that stores files
//! in an embedded database (SQLite by default). It supports multiple independent
//! vaults, content-addressable storage with deduplication, and familiar
//! filesystem operations.
//!
//! # Example
//!
//! ```no_run
//! use std::sync::Arc;
//! use agentvfs::storage::{BackendType, VaultBackend};
//! use agentvfs::fs::FileSystem;
//!
//! // Open a storage backend
//! let backend = Arc::new(
//!     VaultBackend::open(std::path::Path::new("my.avfs"), BackendType::Sqlite).unwrap()
//! );
//!
//! // Create filesystem
//! let fs = FileSystem::new(backend);
//!
//! // Create directories and files
//! fs.create_dir("/docs").unwrap();
//! fs.write_file("/docs/hello.txt", b"Hello, World!").unwrap();
//!
//! // Read file contents
//! let content = fs.read_file("/docs/hello.txt").unwrap();
//! assert_eq!(content, b"Hello, World!");
//! ```

pub mod cache;
pub mod commands;
pub mod error;
pub mod fs;
#[cfg(feature = "fuse")]
pub mod mount;
pub mod runtime;
pub mod shell;
pub mod storage;
pub mod vault;

pub use error::{Result, VfsError};
