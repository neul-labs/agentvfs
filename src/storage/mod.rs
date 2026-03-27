//! Storage backend abstraction layer.
//!
//! This module provides a trait-based abstraction over different storage backends
//! (SQLite, Sled, LMDB, etc.). Each backend implements its native concurrency model.

mod sqlite;

#[cfg(feature = "sled-backend")]
mod sled;

#[cfg(feature = "lmdb-backend")]
mod lmdb;

pub use sqlite::{
    AuditEntry, GcStats, OrphanedBlob, PruneStats, QuotaCheck, QuotaSettings,
    RestoreStats, SnapshotInfo, SqliteBackend, VaultStats,
};

#[cfg(feature = "sled-backend")]
pub use self::sled::SledBackend;

#[cfg(feature = "lmdb-backend")]
pub use self::lmdb::LmdbBackend;

use std::fmt;
use std::str::FromStr;

/// Storage backend type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BackendType {
    /// SQLite backend (default)
    #[default]
    Sqlite,
    /// Sled backend (requires sled-backend feature)
    #[cfg(feature = "sled-backend")]
    Sled,
    /// LMDB backend (requires lmdb-backend feature)
    #[cfg(feature = "lmdb-backend")]
    Lmdb,
}

impl BackendType {
    /// Get all available backend types.
    pub fn available() -> Vec<BackendType> {
        let mut backends = vec![BackendType::Sqlite];
        #[cfg(feature = "sled-backend")]
        backends.push(BackendType::Sled);
        #[cfg(feature = "lmdb-backend")]
        backends.push(BackendType::Lmdb);
        backends
    }

    /// Get the file extension for this backend.
    pub fn extension(&self) -> &'static str {
        match self {
            BackendType::Sqlite => "avfs",
            #[cfg(feature = "sled-backend")]
            BackendType::Sled => "sled",
            #[cfg(feature = "lmdb-backend")]
            BackendType::Lmdb => "lmdb",
        }
    }
}

impl fmt::Display for BackendType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BackendType::Sqlite => write!(f, "sqlite"),
            #[cfg(feature = "sled-backend")]
            BackendType::Sled => write!(f, "sled"),
            #[cfg(feature = "lmdb-backend")]
            BackendType::Lmdb => write!(f, "lmdb"),
        }
    }
}

impl FromStr for BackendType {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "sqlite" => Ok(BackendType::Sqlite),
            #[cfg(feature = "sled-backend")]
            "sled" => Ok(BackendType::Sled),
            #[cfg(feature = "lmdb-backend")]
            "lmdb" => Ok(BackendType::Lmdb),
            _ => Err(format!("unknown backend: {}", s)),
        }
    }
}

use crate::error::Result;

/// Storage backend trait.
///
/// All storage backends must implement this trait. The trait is designed to be
/// backend-agnostic while allowing each backend to use its native concurrency model.
///
/// # Concurrency
///
/// Concurrency behavior depends on the backend:
/// - **SQLite (WAL)**: Concurrent reads, single writer with busy timeout
/// - **Sled**: Lock-free reads, internally serialized writes
/// - **LMDB**: MVCC with single writer
/// - **RocksDB**: Concurrent reads and writes with internal synchronization
pub trait StorageBackend: Send + Sync {
    /// Get a value by key from a collection.
    ///
    /// Returns `Ok(None)` if the key doesn't exist.
    fn get(&self, collection: &str, key: &[u8]) -> Result<Option<Vec<u8>>>;

    /// Store a key-value pair in a collection.
    ///
    /// Overwrites any existing value for the key.
    fn put(&self, collection: &str, key: &[u8], value: &[u8]) -> Result<()>;

    /// Delete a key from a collection.
    ///
    /// Returns `Ok(())` even if the key doesn't exist.
    fn delete(&self, collection: &str, key: &[u8]) -> Result<()>;

    /// Check if a key exists in a collection.
    fn exists(&self, collection: &str, key: &[u8]) -> Result<bool>;

    /// Scan all key-value pairs in a collection.
    fn scan_all(&self, collection: &str) -> Result<Vec<(Vec<u8>, Vec<u8>)>>;

    /// Scan key-value pairs where the key starts with the given prefix.
    fn scan_prefix(&self, collection: &str, prefix: &[u8]) -> Result<Vec<(Vec<u8>, Vec<u8>)>>;

    /// Ensure all pending writes are persisted to disk.
    fn sync(&self) -> Result<()>;

    /// Get the path to the database file.
    fn path(&self) -> &std::path::Path;
}
