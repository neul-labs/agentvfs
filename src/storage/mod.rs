//! Storage backend abstraction layer.
//!
//! This module provides a trait-based abstraction over different storage backends
//! (SQLite, Sled, LMDB, etc.). Each backend implements its native concurrency model.

mod sqlite;

pub use sqlite::{
    AuditEntry, GcStats, OrphanedBlob, PruneStats, QuotaCheck, QuotaSettings,
    RestoreStats, SnapshotInfo, SqliteBackend, VaultStats,
};

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
