//! SQLite storage backend implementation.
//!
//! Uses WAL mode for concurrent reads with single writer.
//! Busy timeout handles write conflicts automatically.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};

use crate::error::{Result, VfsError};
use crate::fs::{FileEntry, FileType};
use crate::storage::StorageBackend;

/// SQLite storage backend.
///
/// # Concurrency Model
///
/// - **Journal Mode**: WAL (Write-Ahead Logging)
/// - **Reads**: Concurrent, non-blocking
/// - **Writes**: Single writer at a time
/// - **Conflict Handling**: busy_timeout (5000ms default) - SQLite retries automatically
/// - **Transactions**: IMMEDIATE mode (acquire write lock at BEGIN)
pub struct SqliteBackend {
    conn: Mutex<Connection>,
    path: PathBuf,
}

impl SqliteBackend {
    /// Open or create a SQLite database at the given path.
    ///
    /// Initializes the database with the vfs schema if it's new.
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;

        // Configure SQLite for concurrent access
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "busy_timeout", 5000)?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;

        let backend = Self {
            conn: Mutex::new(conn),
            path: path.to_path_buf(),
        };

        // Initialize schema if needed
        backend.initialize_schema()?;

        Ok(backend)
    }

    /// Initialize the database schema.
    fn initialize_schema(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        // Check if schema exists
        let has_schema: bool = conn.query_row(
            "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='files'",
            [],
            |row| row.get(0),
        )?;

        if has_schema {
            // Check for schema migration
            drop(conn);
            return self.migrate_schema();
        }

        // Create schema
        conn.execute_batch(
            r#"
            -- File metadata
            CREATE TABLE files (
                id INTEGER PRIMARY KEY,
                parent_id INTEGER REFERENCES files(id) ON DELETE CASCADE,
                name TEXT NOT NULL,
                file_type INTEGER NOT NULL,
                content_hash BLOB,
                size INTEGER NOT NULL DEFAULT 0,
                created_at INTEGER NOT NULL,
                modified_at INTEGER NOT NULL,
                UNIQUE(parent_id, name)
            );

            CREATE INDEX idx_files_parent ON files(parent_id);
            CREATE INDEX idx_files_hash ON files(content_hash);

            -- Path lookup cache
            CREATE TABLE paths (
                path TEXT PRIMARY KEY,
                file_id INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE
            );

            -- Content blobs (content-addressable storage)
            CREATE TABLE contents (
                hash BLOB PRIMARY KEY,
                data BLOB NOT NULL,
                size INTEGER NOT NULL,
                ref_count INTEGER NOT NULL DEFAULT 1
            );

            -- Vault settings
            CREATE TABLE settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            -- Version history for files
            CREATE TABLE file_versions (
                id INTEGER PRIMARY KEY,
                file_id INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
                version_number INTEGER NOT NULL,
                content_hash BLOB NOT NULL,
                size INTEGER NOT NULL,
                created_at INTEGER NOT NULL,
                UNIQUE(file_id, version_number)
            );

            CREATE INDEX idx_versions_file ON file_versions(file_id);
            CREATE INDEX idx_versions_created ON file_versions(created_at);

            -- Full-text search index
            CREATE VIRTUAL TABLE fts_content USING fts5(
                path,
                content,
                tokenize='porter unicode61'
            );

            -- Tags registry
            CREATE TABLE tags (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                created_at INTEGER NOT NULL
            );

            -- File-tag associations (many-to-many)
            CREATE TABLE file_tags (
                id INTEGER PRIMARY KEY,
                file_id INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
                tag_id INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
                created_at INTEGER NOT NULL,
                UNIQUE(file_id, tag_id)
            );

            CREATE INDEX idx_file_tags_file ON file_tags(file_id);
            CREATE INDEX idx_file_tags_tag ON file_tags(tag_id);

            -- File metadata (key-value pairs)
            CREATE TABLE file_metadata (
                id INTEGER PRIMARY KEY,
                file_id INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
                key TEXT NOT NULL,
                value TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                modified_at INTEGER NOT NULL,
                UNIQUE(file_id, key)
            );

            CREATE INDEX idx_metadata_file ON file_metadata(file_id);
            CREATE INDEX idx_metadata_key ON file_metadata(key);

            -- Snapshots for vault state
            CREATE TABLE snapshots (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                created_at INTEGER NOT NULL,
                file_count INTEGER NOT NULL,
                total_size INTEGER NOT NULL,
                description TEXT
            );

            -- Snapshot file entries
            CREATE TABLE snapshot_files (
                id INTEGER PRIMARY KEY,
                snapshot_id INTEGER NOT NULL REFERENCES snapshots(id) ON DELETE CASCADE,
                path TEXT NOT NULL,
                file_type INTEGER NOT NULL,
                content_hash BLOB,
                size INTEGER NOT NULL,
                created_at INTEGER NOT NULL,
                modified_at INTEGER NOT NULL
            );

            CREATE INDEX idx_snapshot_files_snapshot ON snapshot_files(snapshot_id);

            -- Snapshot version history
            CREATE TABLE snapshot_versions (
                id INTEGER PRIMARY KEY,
                snapshot_id INTEGER NOT NULL REFERENCES snapshots(id) ON DELETE CASCADE,
                path TEXT NOT NULL,
                version_number INTEGER NOT NULL,
                content_hash BLOB NOT NULL,
                size INTEGER NOT NULL,
                created_at INTEGER NOT NULL,
                UNIQUE(snapshot_id, path, version_number)
            );

            CREATE INDEX idx_snapshot_versions_snapshot ON snapshot_versions(snapshot_id);

            -- Snapshot tag registry
            CREATE TABLE snapshot_tags (
                id INTEGER PRIMARY KEY,
                snapshot_id INTEGER NOT NULL REFERENCES snapshots(id) ON DELETE CASCADE,
                name TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                UNIQUE(snapshot_id, name)
            );

            CREATE INDEX idx_snapshot_tags_snapshot ON snapshot_tags(snapshot_id);

            -- Snapshot file-tag associations
            CREATE TABLE snapshot_file_tags (
                id INTEGER PRIMARY KEY,
                snapshot_id INTEGER NOT NULL REFERENCES snapshots(id) ON DELETE CASCADE,
                path TEXT NOT NULL,
                tag_name TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                UNIQUE(snapshot_id, path, tag_name)
            );

            CREATE INDEX idx_snapshot_file_tags_snapshot ON snapshot_file_tags(snapshot_id);

            -- Snapshot metadata
            CREATE TABLE snapshot_metadata (
                id INTEGER PRIMARY KEY,
                snapshot_id INTEGER NOT NULL REFERENCES snapshots(id) ON DELETE CASCADE,
                path TEXT NOT NULL,
                key TEXT NOT NULL,
                value TEXT NOT NULL,
                modified_at INTEGER NOT NULL,
                UNIQUE(snapshot_id, path, key)
            );

            CREATE INDEX idx_snapshot_metadata_snapshot ON snapshot_metadata(snapshot_id);

            -- Audit log for operations
            CREATE TABLE audit_log (
                id INTEGER PRIMARY KEY,
                timestamp INTEGER NOT NULL,
                operation TEXT NOT NULL,
                path TEXT,
                details TEXT
            );

            CREATE INDEX idx_audit_timestamp ON audit_log(timestamp);
            CREATE INDEX idx_audit_operation ON audit_log(operation);

            -- Initialize root directory (id=1)
            INSERT INTO files (id, parent_id, name, file_type, size, created_at, modified_at)
            VALUES (1, NULL, '', 1, 0, strftime('%s', 'now'), strftime('%s', 'now'));

            INSERT INTO paths (path, file_id) VALUES ('/', 1);

            INSERT INTO settings (key, value) VALUES
                ('schema_version', '5'),
                ('created_at', strftime('%s', 'now'));
            "#,
        )?;

        Ok(())
    }

    /// Migrate schema from older versions.
    fn migrate_schema(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        // Get current schema version
        let version: String = conn
            .query_row(
                "SELECT value FROM settings WHERE key = 'schema_version'",
                [],
                |row| row.get(0),
            )
            .unwrap_or_else(|_| "1".to_string());

        let version_num: u32 = version.parse().unwrap_or(1);

        if version_num >= 5 {
            return Ok(());
        }

        // Migrate from v3 to v4: Add snapshots, snapshot_files, audit_log tables
        if version_num < 4 {
            conn.execute_batch(
                r#"
                -- Snapshots for vault state
                CREATE TABLE IF NOT EXISTS snapshots (
                    id INTEGER PRIMARY KEY,
                    name TEXT NOT NULL UNIQUE,
                    created_at INTEGER NOT NULL,
                    file_count INTEGER NOT NULL,
                    total_size INTEGER NOT NULL,
                    description TEXT
                );

                -- Snapshot file entries
                CREATE TABLE IF NOT EXISTS snapshot_files (
                    id INTEGER PRIMARY KEY,
                    snapshot_id INTEGER NOT NULL REFERENCES snapshots(id) ON DELETE CASCADE,
                    path TEXT NOT NULL,
                    file_type INTEGER NOT NULL,
                    content_hash BLOB,
                    size INTEGER NOT NULL,
                    created_at INTEGER NOT NULL,
                    modified_at INTEGER NOT NULL
                );

                CREATE INDEX IF NOT EXISTS idx_snapshot_files_snapshot ON snapshot_files(snapshot_id);

                -- Audit log for operations
                CREATE TABLE IF NOT EXISTS audit_log (
                    id INTEGER PRIMARY KEY,
                    timestamp INTEGER NOT NULL,
                    operation TEXT NOT NULL,
                    path TEXT,
                    details TEXT
                );

                CREATE INDEX IF NOT EXISTS idx_audit_timestamp ON audit_log(timestamp);
                CREATE INDEX IF NOT EXISTS idx_audit_operation ON audit_log(operation);

                -- Update schema version
                UPDATE settings SET value = '4' WHERE key = 'schema_version';
                "#,
            )?;
        }

        // Migrate from v4 to v5: Add snapshot state tables for versions, tags, and metadata
        if version_num < 5 {
            conn.execute_batch(
                r#"
                CREATE TABLE IF NOT EXISTS snapshot_versions (
                    id INTEGER PRIMARY KEY,
                    snapshot_id INTEGER NOT NULL REFERENCES snapshots(id) ON DELETE CASCADE,
                    path TEXT NOT NULL,
                    version_number INTEGER NOT NULL,
                    content_hash BLOB NOT NULL,
                    size INTEGER NOT NULL,
                    created_at INTEGER NOT NULL,
                    UNIQUE(snapshot_id, path, version_number)
                );

                CREATE INDEX IF NOT EXISTS idx_snapshot_versions_snapshot ON snapshot_versions(snapshot_id);

                CREATE TABLE IF NOT EXISTS snapshot_tags (
                    id INTEGER PRIMARY KEY,
                    snapshot_id INTEGER NOT NULL REFERENCES snapshots(id) ON DELETE CASCADE,
                    name TEXT NOT NULL,
                    created_at INTEGER NOT NULL,
                    UNIQUE(snapshot_id, name)
                );

                CREATE INDEX IF NOT EXISTS idx_snapshot_tags_snapshot ON snapshot_tags(snapshot_id);

                CREATE TABLE IF NOT EXISTS snapshot_file_tags (
                    id INTEGER PRIMARY KEY,
                    snapshot_id INTEGER NOT NULL REFERENCES snapshots(id) ON DELETE CASCADE,
                    path TEXT NOT NULL,
                    tag_name TEXT NOT NULL,
                    created_at INTEGER NOT NULL,
                    UNIQUE(snapshot_id, path, tag_name)
                );

                CREATE INDEX IF NOT EXISTS idx_snapshot_file_tags_snapshot ON snapshot_file_tags(snapshot_id);

                CREATE TABLE IF NOT EXISTS snapshot_metadata (
                    id INTEGER PRIMARY KEY,
                    snapshot_id INTEGER NOT NULL REFERENCES snapshots(id) ON DELETE CASCADE,
                    path TEXT NOT NULL,
                    key TEXT NOT NULL,
                    value TEXT NOT NULL,
                    modified_at INTEGER NOT NULL,
                    UNIQUE(snapshot_id, path, key)
                );

                CREATE INDEX IF NOT EXISTS idx_snapshot_metadata_snapshot ON snapshot_metadata(snapshot_id);

                UPDATE settings SET value = '5' WHERE key = 'schema_version';
                "#,
            )?;
        }

        Ok(())
    }

    // ==================== File Operations ====================

    /// Get a file entry by path.
    pub fn get_entry_by_path(&self, path: &str) -> Result<FileEntry> {
        let conn = self.conn.lock().unwrap();

        let file_id: i64 = conn
            .query_row("SELECT file_id FROM paths WHERE path = ?", [path], |row| {
                row.get(0)
            })
            .optional()?
            .ok_or_else(|| VfsError::NotFound(PathBuf::from(path)))?;

        self.get_entry_by_id_locked(&conn, file_id)
    }

    /// Get a file entry by ID.
    pub fn get_entry_by_id(&self, id: i64) -> Result<FileEntry> {
        let conn = self.conn.lock().unwrap();
        self.get_entry_by_id_locked(&conn, id)
    }

    fn get_entry_by_id_locked(&self, conn: &Connection, id: i64) -> Result<FileEntry> {
        conn.query_row(
            "SELECT id, parent_id, name, file_type, content_hash, size, created_at, modified_at
             FROM files WHERE id = ?",
            [id],
            |row| {
                Ok(FileEntry {
                    id: row.get(0)?,
                    parent_id: row.get(1)?,
                    name: row.get(2)?,
                    file_type: FileType::from_i64(row.get(3)?).unwrap_or(FileType::File),
                    content_hash: row
                        .get::<_, Option<Vec<u8>>>(4)?
                        .and_then(|v| v.try_into().ok()),
                    size: row.get::<_, i64>(5)? as u64,
                    created_at: chrono::DateTime::from_timestamp(row.get(6)?, 0)
                        .unwrap_or_else(Utc::now),
                    modified_at: chrono::DateTime::from_timestamp(row.get(7)?, 0)
                        .unwrap_or_else(Utc::now),
                })
            },
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => {
                VfsError::Internal(format!("file entry not found: id={}", id))
            }
            e => e.into(),
        })
    }

    /// List children of a directory.
    pub fn list_children(&self, parent_id: i64) -> Result<Vec<FileEntry>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT id, parent_id, name, file_type, content_hash, size, created_at, modified_at
             FROM files WHERE parent_id = ? ORDER BY file_type DESC, name",
        )?;

        let entries = stmt
            .query_map([parent_id], |row| {
                Ok(FileEntry {
                    id: row.get(0)?,
                    parent_id: row.get(1)?,
                    name: row.get(2)?,
                    file_type: FileType::from_i64(row.get(3)?).unwrap_or(FileType::File),
                    content_hash: row
                        .get::<_, Option<Vec<u8>>>(4)?
                        .and_then(|v| v.try_into().ok()),
                    size: row.get::<_, i64>(5)? as u64,
                    created_at: chrono::DateTime::from_timestamp(row.get(6)?, 0)
                        .unwrap_or_else(Utc::now),
                    modified_at: chrono::DateTime::from_timestamp(row.get(7)?, 0)
                        .unwrap_or_else(Utc::now),
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(entries)
    }

    /// Read file content by hash.
    pub fn read_content(&self, hash: &[u8; 32]) -> Result<Vec<u8>> {
        let conn = self.conn.lock().unwrap();

        conn.query_row(
            "SELECT data FROM contents WHERE hash = ?",
            [hash.as_slice()],
            |row| row.get(0),
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => {
                VfsError::Internal("content not found".to_string())
            }
            e => e.into(),
        })
    }

    /// Write file content and get its hash.
    pub fn write_content(&self, data: &[u8]) -> Result<[u8; 32]> {
        use sha2::{Digest, Sha256};

        let mut hasher = Sha256::new();
        hasher.update(data);
        let hash: [u8; 32] = hasher.finalize().into();

        let conn = self.conn.lock().unwrap();
        let size = data.len() as i64;

        conn.execute(
            "INSERT OR IGNORE INTO contents (hash, data, size, ref_count) VALUES (?, ?, ?, 1)",
            params![hash.as_slice(), data, size],
        )?;

        Ok(hash)
    }

    /// Create a file entry.
    pub fn create_file(
        &self,
        parent_id: i64,
        name: &str,
        content_hash: &[u8; 32],
        size: u64,
        path: &str,
    ) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().timestamp();

        conn.execute(
            "INSERT INTO files (parent_id, name, file_type, content_hash, size, created_at, modified_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
            params![
                parent_id,
                name,
                FileType::File.to_i64(),
                content_hash.as_slice(),
                size as i64,
                now,
                now
            ],
        )?;

        let file_id = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO paths (path, file_id) VALUES (?, ?)",
            params![path, file_id],
        )?;

        Ok(file_id)
    }

    /// Update a file's content.
    pub fn update_file(&self, file_id: i64, content_hash: &[u8; 32], size: u64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().timestamp();

        conn.execute(
            "UPDATE files SET content_hash = ?, size = ?, modified_at = ? WHERE id = ?",
            params![content_hash.as_slice(), size as i64, now, file_id],
        )?;

        Ok(())
    }

    /// Create a directory entry.
    pub fn create_directory(&self, parent_id: i64, name: &str, path: &str) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().timestamp();

        conn.execute(
            "INSERT INTO files (parent_id, name, file_type, size, created_at, modified_at)
             VALUES (?, ?, ?, 0, ?, ?)",
            params![parent_id, name, FileType::Directory.to_i64(), now, now],
        )?;

        let dir_id = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO paths (path, file_id) VALUES (?, ?)",
            params![path, dir_id],
        )?;

        Ok(dir_id)
    }

    /// Check if a name exists under a parent.
    pub fn name_exists(&self, parent_id: i64, name: &str) -> Result<bool> {
        let conn = self.conn.lock().unwrap();

        let exists: bool = conn
            .query_row(
                "SELECT 1 FROM files WHERE parent_id = ? AND name = ?",
                params![parent_id, name],
                |_| Ok(true),
            )
            .optional()?
            .unwrap_or(false);

        Ok(exists)
    }

    /// Get file ID by parent and name.
    pub fn get_file_id(&self, parent_id: i64, name: &str) -> Result<Option<i64>> {
        let conn = self.conn.lock().unwrap();

        conn.query_row(
            "SELECT id FROM files WHERE parent_id = ? AND name = ?",
            params![parent_id, name],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| e.into())
    }

    /// Delete a file or directory entry.
    pub fn delete_entry(&self, id: i64, path: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        // Delete paths (including children)
        conn.execute(
            "DELETE FROM paths WHERE path = ? OR path LIKE ?",
            params![path, format!("{}/%", path)],
        )?;

        // Delete file entry (CASCADE handles children)
        conn.execute("DELETE FROM files WHERE id = ?", [id])?;

        Ok(())
    }

    /// Check if a directory has children.
    pub fn has_children(&self, id: i64) -> Result<bool> {
        let conn = self.conn.lock().unwrap();

        let has: bool = conn
            .query_row(
                "SELECT 1 FROM files WHERE parent_id = ? LIMIT 1",
                [id],
                |_| Ok(true),
            )
            .optional()?
            .unwrap_or(false);

        Ok(has)
    }

    /// Move/rename a file entry.
    pub fn move_entry(
        &self,
        id: i64,
        new_parent_id: i64,
        new_name: &str,
        old_path: &str,
        new_path: &str,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().timestamp();

        // Update file entry
        conn.execute(
            "UPDATE files SET parent_id = ?, name = ?, modified_at = ? WHERE id = ?",
            params![new_parent_id, new_name, now, id],
        )?;

        // Delete old paths
        conn.execute(
            "DELETE FROM paths WHERE path = ? OR path LIKE ?",
            params![old_path, format!("{}/%", old_path)],
        )?;

        // Insert new path
        conn.execute(
            "INSERT INTO paths (path, file_id) VALUES (?, ?)",
            params![new_path, id],
        )?;

        Ok(())
    }

    /// Rebuild paths for children after a move.
    pub fn rebuild_child_paths(&self, parent_id: i64, parent_path: &str) -> Result<()> {
        let children = self.list_children(parent_id)?;
        let conn = self.conn.lock().unwrap();
        let mut directories = Vec::new();

        for child in &children {
            let child_path = if parent_path == "/" {
                format!("/{}", child.name)
            } else {
                format!("{}/{}", parent_path, child.name)
            };

            conn.execute(
                "INSERT OR REPLACE INTO paths (path, file_id) VALUES (?, ?)",
                params![&child_path, child.id],
            )?;

            if child.is_dir() {
                directories.push((child.id, child_path));
            }
        }

        drop(conn);

        for (child_id, child_path) in directories {
            self.rebuild_child_paths(child_id, &child_path)?;
        }

        Ok(())
    }

    /// Increment reference count for content.
    pub fn increment_content_ref(&self, hash: &[u8; 32]) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "UPDATE contents SET ref_count = ref_count + 1 WHERE hash = ?",
            [hash.as_slice()],
        )?;

        Ok(())
    }

    /// Copy file entry (shares content).
    pub fn copy_file(
        &self,
        src: &FileEntry,
        new_parent_id: i64,
        new_name: &str,
        new_path: &str,
    ) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().timestamp();

        let hash_slice: Option<&[u8]> = src.content_hash.as_ref().map(|h| h.as_slice());
        conn.execute(
            "INSERT INTO files (parent_id, name, file_type, content_hash, size, created_at, modified_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
            params![
                new_parent_id,
                new_name,
                FileType::File.to_i64(),
                hash_slice,
                src.size as i64,
                now,
                now
            ],
        )?;

        let file_id = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO paths (path, file_id) VALUES (?, ?)",
            params![new_path, file_id],
        )?;

        // Increment ref count
        if let Some(ref hash) = src.content_hash {
            let hash_ref: &[u8] = hash.as_slice();
            conn.execute(
                "UPDATE contents SET ref_count = ref_count + 1 WHERE hash = ?",
                [hash_ref],
            )?;
        }

        Ok(file_id)
    }

    /// Get setting value.
    pub fn get_setting(&self, key: &str) -> Result<Option<String>> {
        let conn = self.conn.lock().unwrap();

        conn.query_row("SELECT value FROM settings WHERE key = ?", [key], |row| {
            row.get(0)
        })
        .optional()
        .map_err(|e| e.into())
    }

    // ==================== Version Operations ====================

    /// Create a version snapshot of the current file state.
    pub fn create_version(&self, file_id: i64, content_hash: &[u8; 32], size: u64) -> Result<u64> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().timestamp();

        // Get next version number
        let next_version: u64 = conn.query_row(
            "SELECT COALESCE(MAX(version_number), 0) + 1 FROM file_versions WHERE file_id = ?",
            [file_id],
            |row| row.get(0),
        )?;

        conn.execute(
            "INSERT INTO file_versions (file_id, version_number, content_hash, size, created_at)
             VALUES (?, ?, ?, ?, ?)",
            params![
                file_id,
                next_version as i64,
                content_hash.as_slice(),
                size as i64,
                now
            ],
        )?;

        Ok(next_version)
    }

    /// Get all versions of a file.
    pub fn get_file_versions(&self, file_id: i64) -> Result<Vec<crate::fs::FileVersion>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT id, file_id, version_number, content_hash, size, created_at
             FROM file_versions WHERE file_id = ? ORDER BY version_number DESC",
        )?;

        let versions = stmt
            .query_map([file_id], |row| {
                Ok(crate::fs::FileVersion {
                    id: row.get(0)?,
                    file_id: row.get(1)?,
                    version_number: row.get::<_, i64>(2)? as u64,
                    content_hash: row.get::<_, Vec<u8>>(3)?.try_into().unwrap_or([0u8; 32]),
                    size: row.get::<_, i64>(4)? as u64,
                    created_at: chrono::DateTime::from_timestamp(row.get(5)?, 0)
                        .unwrap_or_else(Utc::now),
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(versions)
    }

    /// Get a specific version of a file.
    pub fn get_version(&self, file_id: i64, version_num: u64) -> Result<crate::fs::FileVersion> {
        let conn = self.conn.lock().unwrap();

        conn.query_row(
            "SELECT id, file_id, version_number, content_hash, size, created_at
             FROM file_versions WHERE file_id = ? AND version_number = ?",
            params![file_id, version_num as i64],
            |row| {
                Ok(crate::fs::FileVersion {
                    id: row.get(0)?,
                    file_id: row.get(1)?,
                    version_number: row.get::<_, i64>(2)? as u64,
                    content_hash: row.get::<_, Vec<u8>>(3)?.try_into().unwrap_or([0u8; 32]),
                    size: row.get::<_, i64>(4)? as u64,
                    created_at: chrono::DateTime::from_timestamp(row.get(5)?, 0)
                        .unwrap_or_else(Utc::now),
                })
            },
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => {
                VfsError::NotFound(PathBuf::from(format!("version {} not found", version_num)))
            }
            e => e.into(),
        })
    }

    /// Get the content of a specific version.
    pub fn get_version_content(&self, file_id: i64, version_num: u64) -> Result<Vec<u8>> {
        let version = self.get_version(file_id, version_num)?;
        self.read_content(&version.content_hash)
    }

    /// Get the latest version number for a file (0 if no versions).
    pub fn get_latest_version_number(&self, file_id: i64) -> Result<u64> {
        let conn = self.conn.lock().unwrap();

        let version: i64 = conn.query_row(
            "SELECT COALESCE(MAX(version_number), 0) FROM file_versions WHERE file_id = ?",
            [file_id],
            |row| row.get(0),
        )?;

        Ok(version as u64)
    }

    // ==================== Search Operations ====================

    /// Index a file for full-text search.
    pub fn index_file(&self, file_id: i64, path: &str, content: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        // Delete existing index entry if any
        conn.execute("DELETE FROM fts_content WHERE rowid = ?", [file_id])?;

        // Insert new index entry
        conn.execute(
            "INSERT INTO fts_content (rowid, path, content) VALUES (?, ?, ?)",
            params![file_id, path, content],
        )?;

        Ok(())
    }

    /// Remove a file from the search index.
    pub fn remove_from_index(&self, file_id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute("DELETE FROM fts_content WHERE rowid = ?", [file_id])?;

        Ok(())
    }

    /// Rebuild a single file's search index entry from stored content.
    pub fn sync_file_index(&self, file_id: i64, path: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute("DELETE FROM fts_content WHERE rowid = ?", [file_id])?;

        let data: Option<Vec<u8>> = conn
            .query_row(
                "SELECT c.data
                 FROM files f
                 JOIN contents c ON c.hash = f.content_hash
                 WHERE f.id = ? AND f.file_type = 0",
                [file_id],
                |row| row.get(0),
            )
            .optional()?;

        if let Some(data) = data {
            if let Ok(content) = String::from_utf8(data) {
                conn.execute(
                    "INSERT INTO fts_content (rowid, path, content) VALUES (?, ?, ?)",
                    params![file_id, path, content],
                )?;
            }
        }

        Ok(())
    }

    // ==================== Atomic Operations ====================
    /// Atomic file write: stores content, creates or updates file entry,
    /// versions the previous state, and updates the search index in a single
    /// SQLite transaction.
    pub fn write_file_atomic(
        &self,
        parent_id: i64,
        name: &str,
        content: &[u8],
        path: &str,
    ) -> Result<i64> {
        use sha2::{Digest, Sha256};

        let mut hasher = Sha256::new();
        hasher.update(content);
        let hash: [u8; 32] = hasher.finalize().into();
        let size = content.len() as u64;

        let conn = self.conn.lock().unwrap();
        let now = Utc::now().timestamp();

        conn.execute("BEGIN IMMEDIATE", [])?;

        let result = (|| -> Result<i64> {
            // Store content (idempotent)
            conn.execute(
                "INSERT OR IGNORE INTO contents (hash, data, size, ref_count) VALUES (?, ?, ?, 1)",
                params![hash.as_slice(), content, size as i64],
            )?;

            // Check if file already exists
            let existing_id: Option<i64> = conn
                .query_row(
                    "SELECT id FROM files WHERE parent_id = ? AND name = ?",
                    params![parent_id, name],
                    |row| row.get(0),
                )
                .optional()?;

            let file_id = if let Some(id) = existing_id {
                // Snapshot current state into a version
                let current_hash: Option<Vec<u8>> = conn
                    .query_row("SELECT content_hash FROM files WHERE id = ?", [id], |row| {
                        row.get(0)
                    })
                    .optional()?;

                if let Some(h) = current_hash {
                    let next_version: i64 = conn.query_row(
                        "SELECT COALESCE(MAX(version_number), 0) + 1 FROM file_versions WHERE file_id = ?",
                        [id],
                        |row| row.get(0),
                    )?;

                    conn.execute(
                        "INSERT INTO file_versions (file_id, version_number, content_hash, size, created_at)
                         VALUES (?, ?, ?, ?, ?)",
                        params![id, next_version, h.as_slice(), size as i64, now],
                    )?;
                }

                // Update existing file
                conn.execute(
                    "UPDATE files SET content_hash = ?, size = ?, modified_at = ? WHERE id = ?",
                    params![hash.as_slice(), size as i64, now, id],
                )?;

                id
            } else {
                // Create new file entry
                conn.execute(
                    "INSERT INTO files (parent_id, name, file_type, content_hash, size, created_at, modified_at)
                     VALUES (?, ?, ?, ?, ?, ?, ?)",
                    params![parent_id, name, FileType::File.to_i64(), hash.as_slice(), size as i64, now, now],
                )?;

                let file_id = conn.last_insert_rowid();

                conn.execute(
                    "INSERT INTO paths (path, file_id) VALUES (?, ?)",
                    params![path, file_id],
                )?;

                // Create initial version
                conn.execute(
                    "INSERT INTO file_versions (file_id, version_number, content_hash, size, created_at)
                     VALUES (?, 1, ?, ?, ?)",
                    params![file_id, hash.as_slice(), size as i64, now],
                )?;

                file_id
            };

            // Update FTS index: always delete old entry, then re-insert if valid UTF-8
            conn.execute("DELETE FROM fts_content WHERE rowid = ?", [file_id])?;
            if let Ok(text) = String::from_utf8(content.to_vec()) {
                conn.execute(
                    "INSERT INTO fts_content (rowid, path, content) VALUES (?, ?, ?)",
                    params![file_id, path, text],
                )?;
            }

            Ok(file_id)
        })();

        match result {
            Ok(id) => {
                conn.execute("COMMIT", [])?;
                Ok(id)
            }
            Err(e) => {
                let _ = conn.execute("ROLLBACK", []);
                Err(e)
            }
        }
    }

    /// Atomic directory creation: checks for duplicates and creates the
    /// directory entry + path in a single SQLite transaction.
    pub fn create_directory_atomic(&self, parent_id: i64, name: &str, path: &str) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().timestamp();

        conn.execute("BEGIN IMMEDIATE", [])?;

        let result = (|| -> Result<i64> {
            let exists: bool = conn
                .query_row(
                    "SELECT 1 FROM files WHERE parent_id = ? AND name = ?",
                    params![parent_id, name],
                    |_| Ok(true),
                )
                .optional()?
                .unwrap_or(false);

            if exists {
                return Err(VfsError::AlreadyExists(PathBuf::from(path)));
            }

            conn.execute(
                "INSERT INTO files (parent_id, name, file_type, size, created_at, modified_at)
                 VALUES (?, ?, ?, 0, ?, ?)",
                params![parent_id, name, FileType::Directory.to_i64(), now, now],
            )?;

            let dir_id = conn.last_insert_rowid();

            conn.execute(
                "INSERT INTO paths (path, file_id) VALUES (?, ?)",
                params![path, dir_id],
            )?;

            Ok(dir_id)
        })();

        match result {
            Ok(id) => {
                conn.execute("COMMIT", [])?;
                Ok(id)
            }
            Err(e) => {
                let _ = conn.execute("ROLLBACK", []);
                Err(e)
            }
        }
    }

    /// Atomic delete: removes paths and the file entry (with CASCADE) in a
    /// single SQLite transaction.
    pub fn delete_entry_atomic(&self, id: i64, path: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute("BEGIN IMMEDIATE", [])?;

        let result = (|| -> Result<()> {
            conn.execute(
                "DELETE FROM paths WHERE path = ? OR path LIKE ?",
                params![path, format!("{}/%", path)],
            )?;

            conn.execute("DELETE FROM files WHERE id = ?", [id])?;

            Ok(())
        })();

        match result {
            Ok(()) => {
                conn.execute("COMMIT", [])?;
                Ok(())
            }
            Err(e) => {
                let _ = conn.execute("ROLLBACK", []);
                Err(e)
            }
        }
    }

    /// Atomic move: updates the file entry, swaps paths, and rebuilds child
    /// paths inside a single SQLite transaction.
    pub fn move_entry_atomic(
        &self,
        id: i64,
        new_parent_id: i64,
        new_name: &str,
        old_path: &str,
        new_path: &str,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().timestamp();

        conn.execute("BEGIN IMMEDIATE", [])?;

        let result = (|| -> Result<()> {
            // Update file entry
            conn.execute(
                "UPDATE files SET parent_id = ?, name = ?, modified_at = ? WHERE id = ?",
                params![new_parent_id, new_name, now, id],
            )?;

            // Delete old paths
            conn.execute(
                "DELETE FROM paths WHERE path = ? OR path LIKE ?",
                params![old_path, format!("{}/%", old_path)],
            )?;

            // Insert new path for the moved entry itself
            conn.execute(
                "INSERT INTO paths (path, file_id) VALUES (?, ?)",
                params![new_path, id],
            )?;

            // If directory, rebuild all descendant paths
            let is_dir: bool = conn
                .query_row(
                    "SELECT file_type = 1 FROM files WHERE id = ?",
                    [id],
                    |row| row.get::<_, bool>(0),
                )
                .optional()?
                .unwrap_or(false);

            if is_dir {
                Self::rebuild_child_paths_locked(&conn, id, new_path)?;
            }

            Ok(())
        })();

        match result {
            Ok(()) => {
                conn.execute("COMMIT", [])?;
                Ok(())
            }
            Err(e) => {
                let _ = conn.execute("ROLLBACK", []);
                Err(e)
            }
        }
    }

    /// Recursive helper for rebuilding child paths while already holding the
    /// connection lock (used inside a transaction).
    fn rebuild_child_paths_locked(
        conn: &Connection,
        parent_id: i64,
        parent_path: &str,
    ) -> Result<()> {
        let mut stmt = conn
            .prepare("SELECT id, name, file_type FROM files WHERE parent_id = ? ORDER BY name")?;

        let children: Vec<(i64, String, bool)> = stmt
            .query_map([parent_id], |row| {
                let id: i64 = row.get(0)?;
                let name: String = row.get(1)?;
                let file_type: i64 = row.get(2)?;
                Ok((id, name, file_type == 1))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        drop(stmt);

        for (child_id, child_name, is_dir) in children {
            let child_path = if parent_path == "/" {
                format!("/{}", child_name)
            } else {
                format!("{}/{}", parent_path, child_name)
            };

            conn.execute(
                "INSERT OR REPLACE INTO paths (path, file_id) VALUES (?, ?)",
                params![&child_path, child_id],
            )?;

            if is_dir {
                Self::rebuild_child_paths_locked(conn, child_id, &child_path)?;
            }
        }

        Ok(())
    }

    /// Atomic copy: inserts the new file entry, its path, and increments the
    /// content reference count in a single SQLite transaction.
    pub fn copy_file_atomic(
        &self,
        src: &FileEntry,
        new_parent_id: i64,
        new_name: &str,
        new_path: &str,
    ) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().timestamp();

        conn.execute("BEGIN IMMEDIATE", [])?;

        let result = (|| -> Result<i64> {
            let hash_slice: Option<&[u8]> = src.content_hash.as_ref().map(|h| h.as_slice());

            conn.execute(
                "INSERT INTO files (parent_id, name, file_type, content_hash, size, created_at, modified_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?)",
                params![
                    new_parent_id,
                    new_name,
                    FileType::File.to_i64(),
                    hash_slice,
                    src.size as i64,
                    now,
                    now
                ],
            )?;

            let file_id = conn.last_insert_rowid();

            conn.execute(
                "INSERT INTO paths (path, file_id) VALUES (?, ?)",
                params![new_path, file_id],
            )?;

            if let Some(ref hash) = src.content_hash {
                conn.execute(
                    "UPDATE contents SET ref_count = ref_count + 1 WHERE hash = ?",
                    [hash.as_slice()],
                )?;
            }

            Ok(file_id)
        })();

        match result {
            Ok(id) => {
                conn.execute("COMMIT", [])?;
                Ok(id)
            }
            Err(e) => {
                let _ = conn.execute("ROLLBACK", []);
                Err(e)
            }
        }
    }

    /// Search content using FTS5.
    pub fn search_content(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<crate::fs::SearchResult>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT rowid, path, snippet(fts_content, 1, '>>>>', '<<<<', '...', 32) as snippet,
                    rank
             FROM fts_content
             WHERE fts_content MATCH ?
             ORDER BY rank
             LIMIT ?",
        )?;

        let results = stmt
            .query_map(params![query, limit as i64], |row| {
                Ok(crate::fs::SearchResult {
                    file_id: row.get(0)?,
                    path: row.get(1)?,
                    snippet: row.get(2)?,
                    rank: row.get(3)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(results)
    }

    /// Rebuild the entire search index.
    pub fn rebuild_search_index(&self) -> Result<u64> {
        let conn = self.conn.lock().unwrap();

        // Clear existing index
        conn.execute("DELETE FROM fts_content", [])?;

        // Get all files with content
        let mut stmt = conn.prepare(
            "SELECT f.id, p.path, c.data
             FROM files f
             JOIN paths p ON p.file_id = f.id
             JOIN contents c ON c.hash = f.content_hash
             WHERE f.file_type = 0", // Files only
        )?;

        let mut indexed = 0u64;
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            let file_id: i64 = row.get(0)?;
            let path: String = row.get(1)?;
            let data: Vec<u8> = row.get(2)?;

            if let Ok(content) = String::from_utf8(data) {
                conn.execute(
                    "INSERT INTO fts_content (rowid, path, content) VALUES (?, ?, ?)",
                    params![file_id, path, content],
                )?;
                indexed += 1;
            }
        }

        Ok(indexed)
    }

    // ==================== Tag Operations ====================

    /// Create a new tag.
    pub fn create_tag(&self, name: &str) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().timestamp();

        conn.execute(
            "INSERT INTO tags (name, created_at) VALUES (?, ?)",
            params![name, now],
        )?;

        Ok(conn.last_insert_rowid())
    }

    /// Delete a tag (and all associations).
    pub fn delete_tag(&self, tag_id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute("DELETE FROM tags WHERE id = ?", [tag_id])?;

        Ok(())
    }

    /// Rename a tag.
    pub fn rename_tag(&self, tag_id: i64, new_name: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "UPDATE tags SET name = ? WHERE id = ?",
            params![new_name, tag_id],
        )?;

        Ok(())
    }

    /// List all tags in the vault.
    pub fn list_tags(&self) -> Result<Vec<crate::fs::Tag>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare("SELECT id, name, created_at FROM tags ORDER BY name")?;

        let tags = stmt
            .query_map([], |row| {
                Ok(crate::fs::Tag {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    created_at: chrono::DateTime::from_timestamp(row.get(2)?, 0)
                        .unwrap_or_else(Utc::now),
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(tags)
    }

    /// Get a tag by name.
    pub fn get_tag_by_name(&self, name: &str) -> Result<Option<crate::fs::Tag>> {
        let conn = self.conn.lock().unwrap();

        conn.query_row(
            "SELECT id, name, created_at FROM tags WHERE name = ?",
            [name],
            |row| {
                Ok(crate::fs::Tag {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    created_at: chrono::DateTime::from_timestamp(row.get(2)?, 0)
                        .unwrap_or_else(Utc::now),
                })
            },
        )
        .optional()
        .map_err(|e| e.into())
    }

    /// Add a tag to a file.
    pub fn add_tag_to_file(&self, file_id: i64, tag_id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().timestamp();

        conn.execute(
            "INSERT OR IGNORE INTO file_tags (file_id, tag_id, created_at) VALUES (?, ?, ?)",
            params![file_id, tag_id, now],
        )?;

        Ok(())
    }

    /// Remove a tag from a file.
    pub fn remove_tag_from_file(&self, file_id: i64, tag_id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "DELETE FROM file_tags WHERE file_id = ? AND tag_id = ?",
            params![file_id, tag_id],
        )?;

        Ok(())
    }

    /// Get all tags for a file.
    pub fn get_file_tags(&self, file_id: i64) -> Result<Vec<crate::fs::Tag>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT t.id, t.name, t.created_at
             FROM tags t
             JOIN file_tags ft ON ft.tag_id = t.id
             WHERE ft.file_id = ?
             ORDER BY t.name",
        )?;

        let tags = stmt
            .query_map([file_id], |row| {
                Ok(crate::fs::Tag {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    created_at: chrono::DateTime::from_timestamp(row.get(2)?, 0)
                        .unwrap_or_else(Utc::now),
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(tags)
    }

    /// Get all file IDs that have a specific tag.
    pub fn get_files_with_tag(&self, tag_id: i64) -> Result<Vec<i64>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare("SELECT file_id FROM file_tags WHERE tag_id = ?")?;

        let file_ids = stmt
            .query_map([tag_id], |row| row.get(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(file_ids)
    }

    /// Get or create a tag by name.
    pub fn get_or_create_tag(&self, name: &str) -> Result<i64> {
        if let Some(tag) = self.get_tag_by_name(name)? {
            Ok(tag.id)
        } else {
            self.create_tag(name)
        }
    }

    // ==================== Metadata Operations ====================

    /// Set metadata on a file (insert or update).
    pub fn set_metadata(&self, file_id: i64, key: &str, value: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().timestamp();

        conn.execute(
            "INSERT INTO file_metadata (file_id, key, value, created_at, modified_at)
             VALUES (?, ?, ?, ?, ?)
             ON CONFLICT(file_id, key) DO UPDATE SET value = ?, modified_at = ?",
            params![file_id, key, value, now, now, value, now],
        )?;

        Ok(())
    }

    /// Get a single metadata value.
    pub fn get_metadata(&self, file_id: i64, key: &str) -> Result<Option<String>> {
        let conn = self.conn.lock().unwrap();

        conn.query_row(
            "SELECT value FROM file_metadata WHERE file_id = ? AND key = ?",
            params![file_id, key],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| e.into())
    }

    /// Get all metadata for a file.
    pub fn get_all_metadata(&self, file_id: i64) -> Result<Vec<crate::fs::Metadata>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT key, value, modified_at FROM file_metadata WHERE file_id = ? ORDER BY key",
        )?;

        let metadata = stmt
            .query_map([file_id], |row| {
                Ok(crate::fs::Metadata {
                    key: row.get(0)?,
                    value: row.get(1)?,
                    modified_at: chrono::DateTime::from_timestamp(row.get(2)?, 0)
                        .unwrap_or_else(Utc::now),
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(metadata)
    }

    /// Delete a metadata key from a file.
    pub fn delete_metadata(&self, file_id: i64, key: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "DELETE FROM file_metadata WHERE file_id = ? AND key = ?",
            params![file_id, key],
        )?;

        Ok(())
    }

    /// Get all file IDs that have a specific metadata key-value pair.
    pub fn get_files_with_metadata(&self, key: &str, value: &str) -> Result<Vec<i64>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt =
            conn.prepare("SELECT file_id FROM file_metadata WHERE key = ? AND value = ?")?;

        let file_ids = stmt
            .query_map(params![key, value], |row| row.get(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(file_ids)
    }

    // ==================== Maintenance Operations ====================

    /// Get vault storage statistics.
    pub fn get_vault_stats(&self) -> Result<VaultStats> {
        let conn = self.conn.lock().unwrap();

        // Count files (type 0)
        let files: u64 = conn.query_row(
            "SELECT COUNT(*) FROM files WHERE file_type = 0",
            [],
            |row| row.get(0),
        )?;

        // Count directories (type 1, excluding root)
        let directories: u64 = conn.query_row(
            "SELECT COUNT(*) FROM files WHERE file_type = 1 AND id != 1",
            [],
            |row| row.get(0),
        )?;

        // Count total versions
        let total_versions: u64 =
            conn.query_row("SELECT COUNT(*) FROM file_versions", [], |row| row.get(0))?;

        // Count content blobs and total size
        let (content_blobs, total_size_bytes): (u64, u64) = conn.query_row(
            "SELECT COUNT(*), COALESCE(SUM(size), 0) FROM contents",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;

        // Find orphaned blobs (content with ref_count = 0)
        let (orphaned_blobs, orphaned_bytes): (u64, u64) = conn.query_row(
            "SELECT COUNT(*), COALESCE(SUM(size), 0) FROM contents WHERE ref_count = 0",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;

        Ok(VaultStats {
            files,
            directories,
            total_versions,
            content_blobs,
            total_size_bytes,
            orphaned_blobs,
            orphaned_bytes,
        })
    }

    // ==================== Prune Operations ====================

    /// Delete old versions, keeping N most recent.
    pub fn prune_versions_keep(&self, file_id: i64, keep: u64) -> Result<u64> {
        let conn = self.conn.lock().unwrap();

        // First get the version numbers to keep
        let mut stmt = conn.prepare(
            "SELECT version_number FROM file_versions
             WHERE file_id = ?
             ORDER BY version_number DESC
             LIMIT ?",
        )?;

        let versions_to_keep: Vec<i64> = stmt
            .query_map(params![file_id, keep as i64], |row| row.get(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        if versions_to_keep.is_empty() {
            return Ok(0);
        }

        // Delete versions not in the keep list
        let placeholders = versions_to_keep
            .iter()
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(",");
        let sql = format!(
            "DELETE FROM file_versions WHERE file_id = ? AND version_number NOT IN ({})",
            placeholders
        );

        let mut stmt = conn.prepare(&sql)?;

        // Build params: file_id followed by version numbers
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(file_id)];
        for v in &versions_to_keep {
            params_vec.push(Box::new(*v));
        }

        let refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|b| b.as_ref()).collect();
        let deleted = stmt.execute(refs.as_slice())?;

        Ok(deleted as u64)
    }

    /// Delete versions older than a timestamp.
    pub fn prune_versions_older_than(&self, file_id: i64, timestamp: i64) -> Result<u64> {
        let conn = self.conn.lock().unwrap();

        let deleted = conn.execute(
            "DELETE FROM file_versions WHERE file_id = ? AND created_at < ?",
            params![file_id, timestamp],
        )?;

        Ok(deleted as u64)
    }

    /// Prune all files in vault.
    pub fn prune_all_versions(
        &self,
        keep: Option<u64>,
        older_than: Option<i64>,
    ) -> Result<PruneStats> {
        let conn = self.conn.lock().unwrap();

        // Get all file IDs with versions
        let mut stmt = conn.prepare("SELECT DISTINCT file_id FROM file_versions")?;

        let file_ids: Vec<i64> = stmt
            .query_map([], |row| row.get(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        drop(stmt);
        drop(conn);

        let mut total_deleted = 0u64;
        let mut files_processed = 0u64;

        for file_id in file_ids {
            let deleted = if let Some(k) = keep {
                self.prune_versions_keep(file_id, k)?
            } else if let Some(ts) = older_than {
                self.prune_versions_older_than(file_id, ts)?
            } else {
                0
            };

            if deleted > 0 {
                files_processed += 1;
                total_deleted += deleted;
            }
        }

        Ok(PruneStats {
            files_processed,
            versions_deleted: total_deleted,
        })
    }

    /// Count versions that would be pruned with --keep N.
    pub fn count_versions_to_prune_keep(&self, file_id: i64, keep: u64) -> Result<u64> {
        let conn = self.conn.lock().unwrap();

        let total: u64 = conn.query_row(
            "SELECT COUNT(*) FROM file_versions WHERE file_id = ?",
            [file_id],
            |row| row.get(0),
        )?;

        Ok(total.saturating_sub(keep))
    }

    /// Count versions that would be pruned with --older-than.
    pub fn count_versions_to_prune_older(&self, file_id: i64, timestamp: i64) -> Result<u64> {
        let conn = self.conn.lock().unwrap();

        let count: u64 = conn.query_row(
            "SELECT COUNT(*) FROM file_versions WHERE file_id = ? AND created_at < ?",
            params![file_id, timestamp],
            |row| row.get(0),
        )?;

        Ok(count)
    }

    // ==================== Garbage Collection Operations ====================

    /// Recalculate all content reference counts.
    pub fn recalculate_ref_counts(&self) -> Result<u64> {
        let conn = self.conn.lock().unwrap();

        // Update ref_count based on actual references from files and file_versions
        conn.execute(
            "UPDATE contents SET ref_count = (
                SELECT COUNT(*) FROM files WHERE files.content_hash = contents.hash
            ) + (
                SELECT COUNT(*) FROM file_versions WHERE file_versions.content_hash = contents.hash
            ) + (
                SELECT COUNT(*) FROM snapshot_files WHERE snapshot_files.content_hash = contents.hash
            ) + (
                SELECT COUNT(*) FROM snapshot_versions WHERE snapshot_versions.content_hash = contents.hash
            )",
            [],
        )?;

        // Return count of blobs with ref_count = 0
        let orphans: u64 = conn.query_row(
            "SELECT COUNT(*) FROM contents WHERE ref_count = 0",
            [],
            |row| row.get(0),
        )?;

        Ok(orphans)
    }

    /// Find orphaned content blobs (ref_count = 0).
    pub fn find_orphaned_blobs(&self) -> Result<Vec<OrphanedBlob>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare("SELECT hash, size FROM contents WHERE ref_count = 0")?;

        let orphans = stmt
            .query_map([], |row| {
                let hash: Vec<u8> = row.get(0)?;
                Ok(OrphanedBlob {
                    hash: hash.try_into().unwrap_or([0u8; 32]),
                    size: row.get(1)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(orphans)
    }

    /// Delete orphaned content blobs.
    pub fn delete_orphaned_blobs(&self) -> Result<GcStats> {
        // First find what we're deleting for stats
        let orphans = self.find_orphaned_blobs()?;
        let orphans_found = orphans.len() as u64;
        let bytes_freed: u64 = orphans.iter().map(|o| o.size).sum();

        let conn = self.conn.lock().unwrap();

        let deleted = conn.execute("DELETE FROM contents WHERE ref_count = 0", [])?;

        Ok(GcStats {
            orphans_found,
            orphans_deleted: deleted as u64,
            bytes_freed,
        })
    }

    // ==================== Compaction Operations ====================

    /// Run VACUUM to reclaim space.
    pub fn vacuum(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        // Checkpoint WAL first
        conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")?;

        // Run VACUUM
        conn.execute_batch("VACUUM")?;

        // Optimize
        conn.execute_batch("PRAGMA optimize")?;

        Ok(())
    }

    /// Get database file size.
    pub fn get_db_size(&self) -> Result<u64> {
        let metadata = std::fs::metadata(&self.path)?;
        let mut size = metadata.len();

        // Also count WAL and SHM files
        let wal_path = self.path.with_extension("db-wal");
        if let Ok(m) = std::fs::metadata(&wal_path) {
            size += m.len();
        }

        let shm_path = self.path.with_extension("db-shm");
        if let Ok(m) = std::fs::metadata(&shm_path) {
            size += m.len();
        }

        Ok(size)
    }

    /// Get all file IDs (files only, not directories).
    pub fn get_all_file_ids(&self) -> Result<Vec<i64>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare("SELECT id FROM files WHERE file_type = 0")?;

        let ids = stmt
            .query_map([], |row| row.get(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(ids)
    }

    // ==================== Quota Operations ====================

    /// Get a quota setting value.
    pub fn get_quota(&self, key: &str) -> Result<Option<u64>> {
        let setting_key = format!("quota_{}", key);
        match self.get_setting(&setting_key)? {
            Some(val) => Ok(val.parse::<u64>().ok()),
            None => Ok(None),
        }
    }

    /// Set a quota setting value.
    pub fn set_quota(&self, key: &str, value: u64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let setting_key = format!("quota_{}", key);

        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?, ?)",
            params![setting_key, value.to_string()],
        )?;

        Ok(())
    }

    /// Clear a quota setting.
    pub fn clear_quota(&self, key: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let setting_key = format!("quota_{}", key);

        conn.execute("DELETE FROM settings WHERE key = ?", [setting_key])?;

        Ok(())
    }

    /// Check if a write operation is allowed under quota limits.
    pub fn check_quota(&self, new_size: u64, new_file_count: u64) -> Result<QuotaCheck> {
        let stats = self.get_vault_stats()?;

        let max_size_mb = self.get_quota("max_size_mb")?;
        let max_files = self.get_quota("max_files")?;
        let max_file_size_mb = self.get_quota("max_file_size_mb")?;

        let mut check = QuotaCheck {
            allowed: true,
            reason: None,
            current_size: stats.total_size_bytes,
            current_files: stats.files,
            max_size_mb,
            max_files,
            max_file_size_mb,
        };

        // Check max file size (only for single file operations)
        if new_file_count == 1 {
            if let Some(max_mb) = max_file_size_mb {
                let max_bytes = max_mb * 1024 * 1024;
                if new_size > max_bytes {
                    check.allowed = false;
                    check.reason = Some(format!(
                        "file size {} bytes exceeds limit of {} MB",
                        new_size, max_mb
                    ));
                    return Ok(check);
                }
            }
        }

        // Check max files
        if new_file_count > 0 {
            if let Some(max) = max_files {
                let new_total_files = stats.files + new_file_count;
                if new_total_files > max {
                    check.allowed = false;
                    check.reason = Some(format!(
                        "file count {} would exceed limit of {}",
                        new_total_files, max
                    ));
                    return Ok(check);
                }
            }
        }

        // Check max total size
        if let Some(max_mb) = max_size_mb {
            let max_bytes = max_mb * 1024 * 1024;
            let new_total = stats.total_size_bytes + new_size;
            if new_total > max_bytes {
                check.allowed = false;
                check.reason = Some(format!(
                    "total size {} bytes would exceed limit of {} MB",
                    new_total, max_mb
                ));
                return Ok(check);
            }
        }

        Ok(check)
    }

    /// Get all quota settings.
    pub fn get_all_quotas(&self) -> Result<QuotaSettings> {
        Ok(QuotaSettings {
            max_size_mb: self.get_quota("max_size_mb")?,
            max_files: self.get_quota("max_files")?,
            max_file_size_mb: self.get_quota("max_file_size_mb")?,
        })
    }

    // ==================== Audit Log Operations ====================

    /// Log an operation to the audit log.
    pub fn log_operation(&self, op: &str, path: Option<&str>, details: Option<&str>) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().timestamp();

        conn.execute(
            "INSERT INTO audit_log (timestamp, operation, path, details) VALUES (?, ?, ?, ?)",
            params![now, op, path, details],
        )?;

        // Check for auto-rotation
        let max_entries = self.get_audit_max_entries_locked(&conn)?;
        let count: u64 = conn.query_row("SELECT COUNT(*) FROM audit_log", [], |row| row.get(0))?;

        if count > max_entries {
            // Delete oldest 10%
            let to_delete = (max_entries / 10).max(1);
            conn.execute(
                "DELETE FROM audit_log WHERE id IN (
                    SELECT id FROM audit_log ORDER BY timestamp ASC LIMIT ?
                )",
                [to_delete as i64],
            )?;
        }

        Ok(())
    }

    fn get_audit_max_entries_locked(&self, conn: &Connection) -> Result<u64> {
        let result: std::result::Result<String, _> = conn.query_row(
            "SELECT value FROM settings WHERE key = 'audit_max_entries'",
            [],
            |row| row.get(0),
        );

        match result {
            Ok(val) => Ok(val.parse::<u64>().unwrap_or(10000)),
            Err(_) => Ok(10000),
        }
    }

    /// Get audit log entries.
    pub fn get_audit_log(&self, limit: usize, since: Option<i64>) -> Result<Vec<AuditEntry>> {
        let conn = self.conn.lock().unwrap();

        if let Some(ts) = since {
            let mut stmt = conn.prepare(
                "SELECT id, timestamp, operation, path, details
                 FROM audit_log
                 WHERE timestamp >= ?
                 ORDER BY timestamp DESC
                 LIMIT ?",
            )?;

            let entries = stmt
                .query_map(params![ts, limit as i64], |row| {
                    Ok(AuditEntry {
                        id: row.get(0)?,
                        timestamp: row.get(1)?,
                        operation: row.get(2)?,
                        path: row.get(3)?,
                        details: row.get(4)?,
                    })
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?;

            Ok(entries)
        } else {
            let mut stmt = conn.prepare(
                "SELECT id, timestamp, operation, path, details
                 FROM audit_log
                 ORDER BY timestamp DESC
                 LIMIT ?",
            )?;

            let entries = stmt
                .query_map([limit as i64], |row| {
                    Ok(AuditEntry {
                        id: row.get(0)?,
                        timestamp: row.get(1)?,
                        operation: row.get(2)?,
                        path: row.get(3)?,
                        details: row.get(4)?,
                    })
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?;

            Ok(entries)
        }
    }

    /// Clear audit log entries.
    pub fn clear_audit_log(&self, before: Option<i64>) -> Result<u64> {
        let conn = self.conn.lock().unwrap();

        let deleted = if let Some(ts) = before {
            conn.execute("DELETE FROM audit_log WHERE timestamp < ?", [ts])?
        } else {
            conn.execute("DELETE FROM audit_log", [])?
        };

        Ok(deleted as u64)
    }

    /// Get audit log entry count.
    pub fn get_audit_count(&self) -> Result<u64> {
        let conn = self.conn.lock().unwrap();

        let count: u64 = conn.query_row("SELECT COUNT(*) FROM audit_log", [], |row| row.get(0))?;

        Ok(count)
    }

    // ==================== Snapshot Operations ====================

    /// Save a snapshot of the current vault state.
    pub fn save_snapshot(&self, name: &str, description: Option<&str>) -> Result<SnapshotInfo> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().timestamp();

        // Get current file stats
        let (file_count, total_size): (u64, u64) = conn.query_row(
            "SELECT COUNT(*), COALESCE(SUM(size), 0) FROM files WHERE file_type = 0",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;

        // Create snapshot record
        conn.execute(
            "INSERT INTO snapshots (name, created_at, file_count, total_size, description)
             VALUES (?, ?, ?, ?, ?)",
            params![name, now, file_count as i64, total_size as i64, description],
        )?;

        let snapshot_id = conn.last_insert_rowid();

        // Copy all files and directories (except root)
        conn.execute(
            "INSERT INTO snapshot_files (snapshot_id, path, file_type, content_hash, size, created_at, modified_at)
             SELECT ?, p.path, f.file_type, f.content_hash, f.size, f.created_at, f.modified_at
             FROM files f
             JOIN paths p ON p.file_id = f.id
             WHERE f.id != 1",
            [snapshot_id],
        )?;

        conn.execute(
            "INSERT INTO snapshot_versions (snapshot_id, path, version_number, content_hash, size, created_at)
             SELECT ?, p.path, fv.version_number, fv.content_hash, fv.size, fv.created_at
             FROM file_versions fv
             JOIN files f ON f.id = fv.file_id
             JOIN paths p ON p.file_id = f.id",
            [snapshot_id],
        )?;

        conn.execute(
            "INSERT INTO snapshot_tags (snapshot_id, name, created_at)
             SELECT ?, name, created_at
             FROM tags",
            [snapshot_id],
        )?;

        conn.execute(
            "INSERT INTO snapshot_file_tags (snapshot_id, path, tag_name, created_at)
             SELECT ?, p.path, t.name, ft.created_at
             FROM file_tags ft
             JOIN files f ON f.id = ft.file_id
             JOIN paths p ON p.file_id = f.id
             JOIN tags t ON t.id = ft.tag_id",
            [snapshot_id],
        )?;

        conn.execute(
            "INSERT INTO snapshot_metadata (snapshot_id, path, key, value, modified_at)
             SELECT ?, p.path, fm.key, fm.value, fm.modified_at
             FROM file_metadata fm
             JOIN files f ON f.id = fm.file_id
             JOIN paths p ON p.file_id = f.id",
            [snapshot_id],
        )?;

        Ok(SnapshotInfo {
            id: snapshot_id,
            name: name.to_string(),
            created_at: now,
            file_count,
            total_size,
            description: description.map(|s| s.to_string()),
        })
    }

    /// List all snapshots.
    pub fn list_snapshots(&self) -> Result<Vec<SnapshotInfo>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT id, name, created_at, file_count, total_size, description
             FROM snapshots
             ORDER BY created_at DESC",
        )?;

        let snapshots = stmt
            .query_map([], |row| {
                Ok(SnapshotInfo {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    created_at: row.get(2)?,
                    file_count: row.get::<_, i64>(3)? as u64,
                    total_size: row.get::<_, i64>(4)? as u64,
                    description: row.get(5)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(snapshots)
    }

    /// Get a snapshot by name.
    pub fn get_snapshot(&self, name: &str) -> Result<SnapshotInfo> {
        let conn = self.conn.lock().unwrap();

        conn.query_row(
            "SELECT id, name, created_at, file_count, total_size, description
             FROM snapshots WHERE name = ?",
            [name],
            |row| {
                Ok(SnapshotInfo {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    created_at: row.get(2)?,
                    file_count: row.get::<_, i64>(3)? as u64,
                    total_size: row.get::<_, i64>(4)? as u64,
                    description: row.get(5)?,
                })
            },
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => {
                VfsError::NotFound(PathBuf::from(format!("snapshot: {}", name)))
            }
            e => e.into(),
        })
    }

    /// Restore vault to a snapshot state.
    pub fn restore_snapshot(&self, name: &str) -> Result<RestoreStats> {
        // Get snapshot info first
        let snapshot = self.get_snapshot(name)?;

        let conn = self.conn.lock().unwrap();

        // Delete all existing files and paths (except root)
        conn.execute("DELETE FROM paths WHERE path != '/'", [])?;
        conn.execute("DELETE FROM files WHERE id != 1", [])?;
        conn.execute("DELETE FROM tags", [])?;

        // Also clear FTS index
        conn.execute("DELETE FROM fts_content", [])?;

        // Get snapshot files
        let mut stmt = conn.prepare(
            "SELECT path, file_type, content_hash, size, created_at, modified_at
             FROM snapshot_files WHERE snapshot_id = ?",
        )?;

        type SnapshotFileRow = (String, i64, Option<Vec<u8>>, i64, i64, i64);
        let files: Vec<SnapshotFileRow> = stmt
            .query_map([snapshot.id], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        drop(stmt);

        let mut files_restored = 0u64;
        let mut dirs_restored = 0u64;
        let mut files_to_index = Vec::new();

        // Sort by path depth (directories first, then files)
        let mut sorted_files = files.clone();
        sorted_files.sort_by(|a, b| {
            let depth_a = a.0.matches('/').count();
            let depth_b = b.0.matches('/').count();
            depth_a.cmp(&depth_b)
        });

        for (path, file_type, content_hash, size, created_at, modified_at) in sorted_files {
            // Parse path to get parent and name
            let path_parts: Vec<&str> = path.trim_matches('/').split('/').collect();
            let name = path_parts.last().unwrap_or(&"").to_string();

            // Get parent path
            let parent_path = if path_parts.len() <= 1 {
                "/".to_string()
            } else {
                format!("/{}", path_parts[..path_parts.len() - 1].join("/"))
            };

            // Get parent ID
            let parent_id: i64 = conn.query_row(
                "SELECT file_id FROM paths WHERE path = ?",
                [&parent_path],
                |row| row.get(0),
            )?;

            // Insert file entry
            conn.execute(
                "INSERT INTO files (parent_id, name, file_type, content_hash, size, created_at, modified_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?)",
                params![parent_id, name, file_type, content_hash, size, created_at, modified_at],
            )?;

            let file_id = conn.last_insert_rowid();

            // Insert path entry
            conn.execute(
                "INSERT INTO paths (path, file_id) VALUES (?, ?)",
                params![path, file_id],
            )?;

            if file_type == 0 {
                files_restored += 1;
                files_to_index.push((file_id, path));
            } else {
                dirs_restored += 1;
            }
        }

        let mut tag_stmt = conn.prepare(
            "SELECT name, created_at
             FROM snapshot_tags
             WHERE snapshot_id = ?
             ORDER BY name",
        )?;

        let tags: Vec<(String, i64)> = tag_stmt
            .query_map([snapshot.id], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        drop(tag_stmt);

        for (name, created_at) in tags {
            conn.execute(
                "INSERT INTO tags (name, created_at) VALUES (?, ?)",
                params![name, created_at],
            )?;
        }

        let mut version_stmt = conn.prepare(
            "SELECT path, version_number, content_hash, size, created_at
             FROM snapshot_versions
             WHERE snapshot_id = ?
             ORDER BY path, version_number",
        )?;

        let versions: Vec<(String, i64, Vec<u8>, i64, i64)> = version_stmt
            .query_map([snapshot.id], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        drop(version_stmt);

        for (path, version_number, content_hash, size, created_at) in versions {
            let file_id: i64 =
                conn.query_row("SELECT file_id FROM paths WHERE path = ?", [&path], |row| {
                    row.get(0)
                })?;
            conn.execute(
                "INSERT INTO file_versions (file_id, version_number, content_hash, size, created_at)
                 VALUES (?, ?, ?, ?, ?)",
                params![file_id, version_number, content_hash, size, created_at],
            )?;
        }

        let mut metadata_stmt = conn.prepare(
            "SELECT path, key, value, modified_at
             FROM snapshot_metadata
             WHERE snapshot_id = ?
             ORDER BY path, key",
        )?;

        let metadata_rows: Vec<(String, String, String, i64)> = metadata_stmt
            .query_map([snapshot.id], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        drop(metadata_stmt);

        for (path, key, value, modified_at) in metadata_rows {
            let file_id: i64 =
                conn.query_row("SELECT file_id FROM paths WHERE path = ?", [&path], |row| {
                    row.get(0)
                })?;
            conn.execute(
                "INSERT INTO file_metadata (file_id, key, value, created_at, modified_at)
                 VALUES (?, ?, ?, ?, ?)",
                params![file_id, key, value, modified_at, modified_at],
            )?;
        }

        let mut file_tag_stmt = conn.prepare(
            "SELECT path, tag_name, created_at
             FROM snapshot_file_tags
             WHERE snapshot_id = ?
             ORDER BY path, tag_name",
        )?;

        let file_tag_rows: Vec<(String, String, i64)> = file_tag_stmt
            .query_map([snapshot.id], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        drop(file_tag_stmt);

        for (path, tag_name, created_at) in file_tag_rows {
            let file_id: i64 =
                conn.query_row("SELECT file_id FROM paths WHERE path = ?", [&path], |row| {
                    row.get(0)
                })?;
            let tag_id: i64 =
                conn.query_row("SELECT id FROM tags WHERE name = ?", [&tag_name], |row| {
                    row.get(0)
                })?;
            conn.execute(
                "INSERT INTO file_tags (file_id, tag_id, created_at) VALUES (?, ?, ?)",
                params![file_id, tag_id, created_at],
            )?;
        }

        drop(conn);

        for (file_id, path) in files_to_index {
            self.sync_file_index(file_id, &path)?;
        }

        Ok(RestoreStats {
            files_restored,
            dirs_restored,
        })
    }

    /// Delete a snapshot.
    pub fn delete_snapshot(&self, name: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        let deleted = conn.execute("DELETE FROM snapshots WHERE name = ?", [name])?;

        if deleted == 0 {
            return Err(VfsError::NotFound(PathBuf::from(format!(
                "snapshot: {}",
                name
            ))));
        }

        Ok(())
    }

    /// Set a setting value.
    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?, ?)",
            params![key, value],
        )?;

        Ok(())
    }
}

/// Vault storage statistics.
#[derive(Debug, Clone, serde::Serialize)]
pub struct VaultStats {
    pub files: u64,
    pub directories: u64,
    pub total_versions: u64,
    pub content_blobs: u64,
    pub total_size_bytes: u64,
    pub orphaned_blobs: u64,
    pub orphaned_bytes: u64,
}

/// Prune operation statistics.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PruneStats {
    pub files_processed: u64,
    pub versions_deleted: u64,
}

/// Information about an orphaned blob.
#[derive(Debug, Clone)]
pub struct OrphanedBlob {
    pub hash: [u8; 32],
    pub size: u64,
}

/// Garbage collection statistics.
#[derive(Debug, Clone, serde::Serialize)]
pub struct GcStats {
    pub orphans_found: u64,
    pub orphans_deleted: u64,
    pub bytes_freed: u64,
}

/// Quota check result.
#[derive(Debug, Clone, serde::Serialize)]
pub struct QuotaCheck {
    pub allowed: bool,
    pub reason: Option<String>,
    pub current_size: u64,
    pub current_files: u64,
    pub max_size_mb: Option<u64>,
    pub max_files: Option<u64>,
    pub max_file_size_mb: Option<u64>,
}

/// Quota settings.
#[derive(Debug, Clone, serde::Serialize)]
pub struct QuotaSettings {
    pub max_size_mb: Option<u64>,
    pub max_files: Option<u64>,
    pub max_file_size_mb: Option<u64>,
}

/// Audit log entry.
#[derive(Debug, Clone, serde::Serialize)]
pub struct AuditEntry {
    pub id: i64,
    pub timestamp: i64,
    pub operation: String,
    pub path: Option<String>,
    pub details: Option<String>,
}

/// Snapshot information.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SnapshotInfo {
    pub id: i64,
    pub name: String,
    pub created_at: i64,
    pub file_count: u64,
    pub total_size: u64,
    pub description: Option<String>,
}

/// Restore operation statistics.
#[derive(Debug, Clone, serde::Serialize)]
pub struct RestoreStats {
    pub files_restored: u64,
    pub dirs_restored: u64,
}

impl StorageBackend for SqliteBackend {
    fn get(&self, collection: &str, key: &[u8]) -> Result<Option<Vec<u8>>> {
        let conn = self.conn.lock().unwrap();

        match collection {
            "paths" => {
                let key_str = String::from_utf8_lossy(key);
                conn.query_row(
                    "SELECT file_id FROM paths WHERE path = ?",
                    [key_str.as_ref()],
                    |row| {
                        let id: i64 = row.get(0)?;
                        Ok(id.to_be_bytes().to_vec())
                    },
                )
                .optional()
                .map_err(|e| e.into())
            }
            "contents" => conn
                .query_row("SELECT data FROM contents WHERE hash = ?", [key], |row| {
                    row.get(0)
                })
                .optional()
                .map_err(|e| e.into()),
            "settings" => {
                let key_str = String::from_utf8_lossy(key);
                conn.query_row(
                    "SELECT value FROM settings WHERE key = ?",
                    [key_str.as_ref()],
                    |row| {
                        let val: String = row.get(0)?;
                        Ok(val.into_bytes())
                    },
                )
                .optional()
                .map_err(|e| e.into())
            }
            _ => Err(VfsError::Internal(format!(
                "unknown collection: {}",
                collection
            ))),
        }
    }

    fn put(&self, collection: &str, key: &[u8], value: &[u8]) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        match collection {
            "paths" => {
                let key_str = String::from_utf8_lossy(key);
                let file_id = i64::from_be_bytes(
                    value
                        .try_into()
                        .map_err(|_| VfsError::Internal("invalid value format".to_string()))?,
                );
                conn.execute(
                    "INSERT OR REPLACE INTO paths (path, file_id) VALUES (?, ?)",
                    params![key_str.as_ref(), file_id],
                )?;
            }
            "settings" => {
                let key_str = String::from_utf8_lossy(key);
                let val_str = String::from_utf8_lossy(value);
                conn.execute(
                    "INSERT OR REPLACE INTO settings (key, value) VALUES (?, ?)",
                    params![key_str.as_ref(), val_str.as_ref()],
                )?;
            }
            "contents" => {
                let size = value.len() as i64;
                conn.execute(
                    "INSERT OR IGNORE INTO contents (hash, data, size, ref_count) VALUES (?, ?, ?, 1)",
                    params![key, value, size],
                )?;
            }
            _ => {
                return Err(VfsError::Internal(format!(
                    "put not supported for collection: {}",
                    collection
                )))
            }
        }

        Ok(())
    }

    fn delete(&self, collection: &str, key: &[u8]) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        match collection {
            "paths" => {
                let key_str = String::from_utf8_lossy(key);
                conn.execute("DELETE FROM paths WHERE path = ?", [key_str.as_ref()])?;
            }
            "settings" => {
                let key_str = String::from_utf8_lossy(key);
                conn.execute("DELETE FROM settings WHERE key = ?", [key_str.as_ref()])?;
            }
            "contents" => {
                conn.execute("DELETE FROM contents WHERE hash = ?", [key])?;
            }
            "files" => {
                let id = i64::from_be_bytes(
                    key.try_into()
                        .map_err(|_| VfsError::Internal("invalid key format".to_string()))?,
                );
                conn.execute("DELETE FROM files WHERE id = ?", [id])?;
            }
            _ => {
                return Err(VfsError::Internal(format!(
                    "unknown collection: {}",
                    collection
                )))
            }
        }

        Ok(())
    }

    fn exists(&self, collection: &str, key: &[u8]) -> Result<bool> {
        let conn = self.conn.lock().unwrap();

        match collection {
            "paths" => {
                let key_str = String::from_utf8_lossy(key);
                let exists = conn
                    .query_row(
                        "SELECT 1 FROM paths WHERE path = ?",
                        [key_str.as_ref()],
                        |_| Ok(true),
                    )
                    .optional()?
                    .unwrap_or(false);
                Ok(exists)
            }
            "contents" => {
                let exists = conn
                    .query_row("SELECT 1 FROM contents WHERE hash = ?", [key], |_| Ok(true))
                    .optional()?
                    .unwrap_or(false);
                Ok(exists)
            }
            _ => Err(VfsError::Internal(format!(
                "exists not supported for: {}",
                collection
            ))),
        }
    }

    fn scan_all(&self, collection: &str) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        let conn = self.conn.lock().unwrap();
        let mut results = Vec::new();

        match collection {
            "paths" => {
                let mut stmt = conn.prepare("SELECT path, file_id FROM paths")?;
                let rows = stmt.query_map([], |row| {
                    let path: String = row.get(0)?;
                    let file_id: i64 = row.get(1)?;
                    Ok((path.into_bytes(), file_id.to_be_bytes().to_vec()))
                })?;
                for row in rows {
                    results.push(row?);
                }
            }
            "settings" => {
                let mut stmt = conn.prepare("SELECT key, value FROM settings")?;
                let rows = stmt.query_map([], |row| {
                    let key: String = row.get(0)?;
                    let value: String = row.get(1)?;
                    Ok((key.into_bytes(), value.into_bytes()))
                })?;
                for row in rows {
                    results.push(row?);
                }
            }
            _ => {
                return Err(VfsError::Internal(format!(
                    "scan_all not supported for: {}",
                    collection
                )))
            }
        }

        Ok(results)
    }

    fn scan_prefix(&self, collection: &str, prefix: &[u8]) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        let conn = self.conn.lock().unwrap();
        let mut results = Vec::new();

        match collection {
            "paths" => {
                let prefix_str = String::from_utf8_lossy(prefix);
                let pattern = format!("{}%", prefix_str);
                let mut stmt = conn.prepare("SELECT path, file_id FROM paths WHERE path LIKE ?")?;
                let rows = stmt.query_map([&pattern], |row| {
                    let path: String = row.get(0)?;
                    let file_id: i64 = row.get(1)?;
                    Ok((path.into_bytes(), file_id.to_be_bytes().to_vec()))
                })?;
                for row in rows {
                    results.push(row?);
                }
            }
            _ => {
                return Err(VfsError::Internal(format!(
                    "scan_prefix not supported for: {}",
                    collection
                )))
            }
        }

        Ok(results)
    }

    fn sync(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")?;
        Ok(())
    }

    fn path(&self) -> &Path {
        &self.path
    }
}
