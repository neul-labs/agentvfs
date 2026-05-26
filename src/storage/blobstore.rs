//! Shared content-addressed blob store.
//!
//! In shared-blob storage mode, content blobs live once per *store* (a single `blobs.avfs`
//! beside the `vaults/` directory) instead of inside each vault's `.avfs` file. This makes
//! `fork` O(metadata): a forked vault copies only its (small) metadata database and references
//! the same blobs by SHA-256 hash, deduplicated across all vaults in the store.
//!
//! Crash-consistency: callers write a blob here (committed immediately on this connection)
//! *before* recording the referencing metadata in the vault database. A crash in between leaves
//! at most an unreferenced ("orphan") blob, which is harmless and reclaimed by mark-sweep GC —
//! never a dangling reference.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock, Weak};

use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};

use crate::error::{Result, VfsError};

/// Process-global registry of open blob stores, so every vault in a store shares one handle.
/// Weak references let a store drop once no vault holds it.
static REGISTRY: OnceLock<Mutex<HashMap<PathBuf, Weak<BlobStore>>>> = OnceLock::new();

/// A shared content-addressed blob store backed by its own SQLite database.
pub struct BlobStore {
    conn: Mutex<Connection>,
    #[allow(dead_code)]
    path: PathBuf,
}

impl BlobStore {
    /// Open (creating if needed) the shared blob store for the store rooted at `base_dir`.
    /// Returns a shared handle from the process-global registry.
    pub fn open_shared(base_dir: &Path) -> Result<Arc<BlobStore>> {
        let path = base_dir.join("blobs.avfs");
        let registry = REGISTRY.get_or_init(|| Mutex::new(HashMap::new()));
        let mut map = registry.lock().unwrap();

        if let Some(weak) = map.get(&path) {
            if let Some(arc) = weak.upgrade() {
                return Ok(arc);
            }
        }
        let store = Arc::new(BlobStore::open(&path)?);
        map.insert(path, Arc::downgrade(&store));
        // Prune dead entries opportunistically.
        map.retain(|_, w| w.strong_count() > 0);
        Ok(store)
    }

    fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "busy_timeout", 5000)?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS contents (
                hash BLOB PRIMARY KEY,
                data BLOB NOT NULL,
                size INTEGER NOT NULL,
                created_at INTEGER NOT NULL
            );",
        )?;
        Ok(Self {
            conn: Mutex::new(conn),
            path: path.to_path_buf(),
        })
    }

    /// Store content (idempotent by hash) and return its SHA-256 hash. Commits immediately.
    pub fn write(&self, data: &[u8]) -> Result<[u8; 32]> {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(data);
        let hash: [u8; 32] = hasher.finalize().into();
        self.put_raw(hash.as_slice(), data)?;
        Ok(hash)
    }

    /// Read content by hash.
    pub fn read(&self, hash: &[u8; 32]) -> Result<Vec<u8>> {
        self.get_raw(hash.as_slice())?
            .ok_or_else(|| VfsError::Internal("content not found".to_string()))
    }

    /// Whether a blob with the given 32-byte hash exists.
    pub fn exists(&self, hash: &[u8; 32]) -> Result<bool> {
        self.exists_raw(hash.as_slice())
    }

    /// Raw insert keyed by an arbitrary-length hash slice (used by the KV `StorageBackend` API).
    pub fn put_raw(&self, hash: &[u8], data: &[u8]) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO contents (hash, data, size, created_at) VALUES (?, ?, ?, ?)",
            params![hash, data, data.len() as i64, Utc::now().timestamp()],
        )?;
        Ok(())
    }

    /// Raw lookup by hash slice.
    pub fn get_raw(&self, hash: &[u8]) -> Result<Option<Vec<u8>>> {
        let conn = self.conn.lock().unwrap();
        conn.query_row("SELECT data FROM contents WHERE hash = ?", [hash], |row| {
            row.get(0)
        })
        .optional()
        .map_err(Into::into)
    }

    /// Raw existence check by hash slice.
    pub fn exists_raw(&self, hash: &[u8]) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        Ok(conn
            .query_row("SELECT 1 FROM contents WHERE hash = ?", [hash], |_| {
                Ok(true)
            })
            .optional()?
            .unwrap_or(false))
    }

    /// Raw delete by hash slice.
    pub fn delete_raw(&self, hash: &[u8]) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM contents WHERE hash = ?", [hash])?;
        Ok(())
    }

    /// (blob count, total bytes) across the whole store.
    pub fn stats(&self) -> Result<(u64, u64)> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT COUNT(*), COALESCE(SUM(size), 0) FROM contents",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(Into::into)
    }

    /// Mark-sweep garbage collection: identify (and unless `dry_run`, delete) every blob whose
    /// hash is not in `live`, skipping blobs created within `grace_secs` (to avoid racing a
    /// concurrent writer that has stored a blob but not yet committed its referencing metadata).
    /// Returns (orphaned blobs, bytes) — the amount deleted, or that would be deleted if dry-run.
    pub fn delete_missing(
        &self,
        live: &HashSet<Vec<u8>>,
        grace_secs: i64,
        dry_run: bool,
    ) -> Result<(u64, u64)> {
        let conn = self.conn.lock().unwrap();
        let cutoff = Utc::now().timestamp() - grace_secs;

        let rows: Vec<(Vec<u8>, i64, i64)> = {
            let mut stmt = conn.prepare("SELECT hash, size, created_at FROM contents")?;
            let collected = stmt
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            collected
        };

        let mut orphans = 0u64;
        let mut bytes = 0u64;
        for (hash, size, created_at) in rows {
            if created_at > cutoff {
                continue; // within grace window
            }
            if !live.contains(&hash) {
                if !dry_run {
                    conn.execute("DELETE FROM contents WHERE hash = ?", [hash.as_slice()])?;
                }
                orphans += 1;
                bytes += size as u64;
            }
        }
        Ok((orphans, bytes))
    }
}
