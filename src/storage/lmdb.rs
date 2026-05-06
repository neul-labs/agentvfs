//! LMDB storage backend implementation with Tantivy full-text search.
//!
//! LMDB is a memory-mapped, transactional key-value store with single-writer MVCC.
//! Tantivy provides full-text search capabilities.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Mutex, RwLock};

use chrono::{DateTime, TimeZone, Utc};
use heed::types::Bytes;
use heed::{Database, Env, EnvOpenOptions};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::{Field, Schema, Value, STORED, TEXT};
use tantivy::{doc, Index, IndexReader, IndexWriter, ReloadPolicy};

use crate::error::{Result, VfsError};
use crate::fs::{DirEntry, FileEntry, FileType, FileVersion, Metadata, SearchResult, Tag};
use crate::storage::StorageBackend;

// Re-export shared types from sqlite module for compatibility
pub use super::{
    AuditEntry, GcStats, OrphanedBlob, PruneStats, QuotaCheck, QuotaSettings, RestoreStats,
    SnapshotInfo, VaultStats,
};

/// Database names for different data types.
const DB_FILES: &str = "files";
const DB_FILE_INDEX: &str = "file_index"; // Secondary index: parent_id:name -> file_id

/// Deferred index operation for batched Tantivy commits.
#[derive(Debug)]
enum IndexOp {
    Add { path: String, content: String },
    Remove { path: String },
}
const DB_PATHS: &str = "paths";
const DB_CONTENTS: &str = "contents";
const DB_VERSIONS: &str = "versions";
const DB_TAGS: &str = "tags";
const DB_FILE_TAGS: &str = "file_tags";
const DB_METADATA: &str = "metadata";
const DB_SNAPSHOTS: &str = "snapshots";
const DB_SNAPSHOT_FILES: &str = "snapshot_files";
const DB_AUDIT: &str = "audit";
const DB_SETTINGS: &str = "settings";
const DB_COUNTERS: &str = "counters";

/// LMDB storage backend.
///
/// # Concurrency Model
///
/// - **Reads**: Multiple concurrent readers via MVCC
/// - **Writes**: Single writer (enforced by LMDB)
/// - **Transactions**: Explicit read/write transactions
/// - **Memory**: Uses memory-mapped files for zero-copy reads
pub struct LmdbBackend {
    env: Env,
    path: PathBuf,
    // Named databases (one per collection)
    db_files: Database<Bytes, Bytes>,
    db_paths: Database<Bytes, Bytes>,
    db_contents: Database<Bytes, Bytes>,
    db_versions: Database<Bytes, Bytes>,
    db_tags: Database<Bytes, Bytes>,
    db_file_tags: Database<Bytes, Bytes>,
    db_metadata: Database<Bytes, Bytes>,
    db_snapshots: Database<Bytes, Bytes>,
    db_snapshot_files: Database<Bytes, Bytes>,
    db_audit: Database<Bytes, Bytes>,
    db_settings: Database<Bytes, Bytes>,
    db_counters: Database<Bytes, Bytes>,
    db_file_index: Database<Bytes, Bytes>, // Secondary index: parent_id:name -> file_id
    // ID counters
    next_file_id: AtomicI64,
    next_version_id: AtomicI64,
    next_tag_id: AtomicI64,
    next_snapshot_id: AtomicI64,
    next_audit_id: AtomicI64,
    // Tantivy search
    index: Index,
    index_writer: RwLock<IndexWriter>,
    index_reader: IndexReader,
    field_path: Field,
    field_content: Field,
    // Deferred index operations for batched commits
    pending_index_ops: Mutex<Vec<IndexOp>>,
}

/// Stored file entry in LMDB (serializable version).
#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredFileEntry {
    id: i64,
    parent_id: Option<i64>,
    name: String,
    file_type: u8,
    content_hash: Option<[u8; 32]>,
    size: u64,
    created_at: i64,
    modified_at: i64,
}

impl From<StoredFileEntry> for FileEntry {
    fn from(stored: StoredFileEntry) -> Self {
        Self {
            id: stored.id,
            parent_id: stored.parent_id,
            name: stored.name,
            file_type: FileType::from_i64(stored.file_type as i64).unwrap_or(FileType::File),
            content_hash: stored.content_hash,
            size: stored.size,
            created_at: Utc.timestamp_opt(stored.created_at, 0).unwrap(),
            modified_at: Utc.timestamp_opt(stored.modified_at, 0).unwrap(),
        }
    }
}

impl From<&FileEntry> for StoredFileEntry {
    fn from(entry: &FileEntry) -> Self {
        Self {
            id: entry.id,
            parent_id: entry.parent_id,
            name: entry.name.clone(),
            file_type: entry.file_type as u8,
            content_hash: entry.content_hash,
            size: entry.size,
            created_at: entry.created_at.timestamp(),
            modified_at: entry.modified_at.timestamp(),
        }
    }
}

/// Stored version entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredVersion {
    id: i64,
    file_id: i64,
    version_number: u64,
    content_hash: [u8; 32],
    size: u64,
    created_at: i64,
}

impl From<StoredVersion> for FileVersion {
    fn from(stored: StoredVersion) -> Self {
        Self {
            id: stored.id,
            file_id: stored.file_id,
            version_number: stored.version_number,
            content_hash: stored.content_hash,
            size: stored.size,
            created_at: Utc.timestamp_opt(stored.created_at, 0).unwrap(),
        }
    }
}

/// Stored tag.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredTag {
    id: i64,
    name: String,
    created_at: i64,
}

impl From<StoredTag> for Tag {
    fn from(stored: StoredTag) -> Self {
        Self {
            id: stored.id,
            name: stored.name,
            created_at: Utc.timestamp_opt(stored.created_at, 0).unwrap(),
        }
    }
}

/// Stored metadata entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredMetadata {
    file_id: i64,
    key: String,
    value: String,
    created_at: i64,
    modified_at: i64,
}

/// Stored snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredSnapshot {
    id: i64,
    name: String,
    created_at: i64,
    file_count: i64,
    total_size: i64,
    description: Option<String>,
}

impl From<StoredSnapshot> for SnapshotInfo {
    fn from(stored: StoredSnapshot) -> Self {
        Self {
            id: stored.id,
            name: stored.name,
            created_at: stored.created_at,
            file_count: stored.file_count as u64,
            total_size: stored.total_size as u64,
            description: stored.description,
        }
    }
}

/// Stored snapshot file.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredSnapshotFile {
    snapshot_id: i64,
    path: String,
    file_type: u8,
    content_hash: Option<[u8; 32]>,
    size: u64,
    created_at: i64,
    modified_at: i64,
}

/// Stored audit entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredAuditEntry {
    id: i64,
    timestamp: i64,
    operation: String,
    path: Option<String>,
    details: Option<String>,
}

impl From<StoredAuditEntry> for AuditEntry {
    fn from(stored: StoredAuditEntry) -> Self {
        Self {
            id: stored.id,
            timestamp: stored.timestamp,
            operation: stored.operation,
            path: stored.path,
            details: stored.details,
        }
    }
}

/// Content blob entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredContent {
    hash: [u8; 32],
    data: Vec<u8>,
    size: u64,
    ref_count: u32,
}

impl LmdbBackend {
    /// Open or create an LMDB database at the given path.
    pub fn open(path: &Path) -> Result<Self> {
        // Create directory if it doesn't exist
        std::fs::create_dir_all(path)?;

        // Configure LMDB environment
        let env = unsafe {
            EnvOpenOptions::new()
                .map_size(10 * 1024 * 1024 * 1024) // 10GB max size
                .max_dbs(20) // We need ~12 databases
                .open(path)?
        };

        // Create/open all databases
        let mut wtxn = env.write_txn()?;

        let db_files = env.create_database(&mut wtxn, Some(DB_FILES))?;
        let db_paths = env.create_database(&mut wtxn, Some(DB_PATHS))?;
        let db_contents = env.create_database(&mut wtxn, Some(DB_CONTENTS))?;
        let db_versions = env.create_database(&mut wtxn, Some(DB_VERSIONS))?;
        let db_tags = env.create_database(&mut wtxn, Some(DB_TAGS))?;
        let db_file_tags = env.create_database(&mut wtxn, Some(DB_FILE_TAGS))?;
        let db_metadata = env.create_database(&mut wtxn, Some(DB_METADATA))?;
        let db_snapshots = env.create_database(&mut wtxn, Some(DB_SNAPSHOTS))?;
        let db_snapshot_files = env.create_database(&mut wtxn, Some(DB_SNAPSHOT_FILES))?;
        let db_audit = env.create_database(&mut wtxn, Some(DB_AUDIT))?;
        let db_settings = env.create_database(&mut wtxn, Some(DB_SETTINGS))?;
        let db_counters = env.create_database(&mut wtxn, Some(DB_COUNTERS))?;
        let db_file_index = env.create_database(&mut wtxn, Some(DB_FILE_INDEX))?;

        wtxn.commit()?;

        // Create tantivy index in a subdirectory
        let index_path = path.with_extension("lmdb.tantivy");
        std::fs::create_dir_all(&index_path)?;

        // Build tantivy schema
        let mut schema_builder = Schema::builder();
        let field_path = schema_builder.add_text_field("path", TEXT | STORED);
        let field_content = schema_builder.add_text_field("content", TEXT);
        let schema = schema_builder.build();

        // Open or create index
        let index = if index_path.join("meta.json").exists() {
            Index::open_in_dir(&index_path)?
        } else {
            Index::create_in_dir(&index_path, schema.clone())?
        };

        let index_writer = index.writer(50_000_000)?; // 50MB heap
        let index_reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()?;

        let backend = Self {
            env,
            path: path.to_path_buf(),
            db_files,
            db_paths,
            db_contents,
            db_versions,
            db_tags,
            db_file_tags,
            db_metadata,
            db_snapshots,
            db_snapshot_files,
            db_audit,
            db_settings,
            db_counters,
            db_file_index,
            next_file_id: AtomicI64::new(1),
            next_version_id: AtomicI64::new(1),
            next_tag_id: AtomicI64::new(1),
            next_snapshot_id: AtomicI64::new(1),
            next_audit_id: AtomicI64::new(1),
            index,
            index_writer: RwLock::new(index_writer),
            index_reader,
            field_path,
            field_content,
            pending_index_ops: Mutex::new(Vec::new()),
        };

        backend.initialize()?;
        Ok(backend)
    }

    /// Initialize the database with root directory and load counters.
    fn initialize(&self) -> Result<()> {
        // Load counters from db
        self.load_counters()?;

        // Check if root directory exists
        let rtxn = self.env.read_txn()?;
        let root_exists = self.db_paths.get(&rtxn, b"/")?.is_some();
        drop(rtxn);

        if !root_exists {
            // Create root directory
            let root_id = self.next_id(&self.next_file_id);
            let now = Utc::now();
            let root = StoredFileEntry {
                id: root_id,
                parent_id: None,
                name: String::new(),
                file_type: FileType::Directory as u8,
                content_hash: None,
                size: 0,
                created_at: now.timestamp(),
                modified_at: now.timestamp(),
            };

            let mut wtxn = self.env.write_txn()?;
            self.db_files.put(
                &mut wtxn,
                &root_id.to_be_bytes(),
                &serde_json::to_vec(&root)?,
            )?;
            self.db_paths.put(&mut wtxn, b"/", &root_id.to_be_bytes())?;

            // Set created_at setting
            self.db_settings.put(
                &mut wtxn,
                b"created_at",
                now.timestamp().to_string().as_bytes(),
            )?;
            self.db_settings.put(&mut wtxn, b"schema_version", b"1")?;

            wtxn.commit()?;
            self.save_counters()?;
        }

        Ok(())
    }

    /// Load ID counters from database.
    fn load_counters(&self) -> Result<()> {
        let rtxn = self.env.read_txn()?;

        if let Some(v) = self.db_counters.get(&rtxn, b"file_id")? {
            let id = i64::from_be_bytes(v.try_into().unwrap_or([0; 8]));
            self.next_file_id.store(id, Ordering::SeqCst);
        }
        if let Some(v) = self.db_counters.get(&rtxn, b"version_id")? {
            let id = i64::from_be_bytes(v.try_into().unwrap_or([0; 8]));
            self.next_version_id.store(id, Ordering::SeqCst);
        }
        if let Some(v) = self.db_counters.get(&rtxn, b"tag_id")? {
            let id = i64::from_be_bytes(v.try_into().unwrap_or([0; 8]));
            self.next_tag_id.store(id, Ordering::SeqCst);
        }
        if let Some(v) = self.db_counters.get(&rtxn, b"snapshot_id")? {
            let id = i64::from_be_bytes(v.try_into().unwrap_or([0; 8]));
            self.next_snapshot_id.store(id, Ordering::SeqCst);
        }
        if let Some(v) = self.db_counters.get(&rtxn, b"audit_id")? {
            let id = i64::from_be_bytes(v.try_into().unwrap_or([0; 8]));
            self.next_audit_id.store(id, Ordering::SeqCst);
        }

        Ok(())
    }

    /// Save ID counters to database.
    fn save_counters(&self) -> Result<()> {
        let mut wtxn = self.env.write_txn()?;

        self.db_counters.put(
            &mut wtxn,
            b"file_id",
            &self.next_file_id.load(Ordering::SeqCst).to_be_bytes(),
        )?;
        self.db_counters.put(
            &mut wtxn,
            b"version_id",
            &self.next_version_id.load(Ordering::SeqCst).to_be_bytes(),
        )?;
        self.db_counters.put(
            &mut wtxn,
            b"tag_id",
            &self.next_tag_id.load(Ordering::SeqCst).to_be_bytes(),
        )?;
        self.db_counters.put(
            &mut wtxn,
            b"snapshot_id",
            &self.next_snapshot_id.load(Ordering::SeqCst).to_be_bytes(),
        )?;
        self.db_counters.put(
            &mut wtxn,
            b"audit_id",
            &self.next_audit_id.load(Ordering::SeqCst).to_be_bytes(),
        )?;

        wtxn.commit()?;
        Ok(())
    }

    /// Get next ID from an atomic counter.
    fn next_id(&self, counter: &AtomicI64) -> i64 {
        counter.fetch_add(1, Ordering::SeqCst)
    }

    /// Get database handle by collection name.
    fn get_database(&self, collection: &str) -> Result<Database<Bytes, Bytes>> {
        match collection {
            DB_FILES => Ok(self.db_files),
            DB_PATHS => Ok(self.db_paths),
            DB_CONTENTS => Ok(self.db_contents),
            DB_VERSIONS => Ok(self.db_versions),
            DB_TAGS => Ok(self.db_tags),
            DB_FILE_TAGS => Ok(self.db_file_tags),
            DB_METADATA => Ok(self.db_metadata),
            DB_SNAPSHOTS => Ok(self.db_snapshots),
            DB_SNAPSHOT_FILES => Ok(self.db_snapshot_files),
            DB_AUDIT => Ok(self.db_audit),
            DB_SETTINGS => Ok(self.db_settings),
            DB_COUNTERS => Ok(self.db_counters),
            _ => Err(VfsError::Internal(format!(
                "unknown collection: {}",
                collection
            ))),
        }
    }

    // =========================================================================
    // File Operations
    // =========================================================================

    /// Get a file entry by its path.
    pub fn get_entry_by_path(&self, path: &str) -> Result<FileEntry> {
        let rtxn = self.env.read_txn()?;

        let file_id_bytes = self
            .db_paths
            .get(&rtxn, path.as_bytes())?
            .ok_or_else(|| VfsError::NotFound(path.into()))?;

        let file_id = i64::from_be_bytes(file_id_bytes.try_into().unwrap_or([0; 8]));

        let entry_bytes = self
            .db_files
            .get(&rtxn, &file_id.to_be_bytes())?
            .ok_or_else(|| VfsError::NotFound(path.into()))?;

        let stored: StoredFileEntry = serde_json::from_slice(entry_bytes)?;
        Ok(stored.into())
    }

    /// Get a file entry by its ID.
    pub fn get_entry_by_id(&self, id: i64) -> Result<Option<FileEntry>> {
        let rtxn = self.env.read_txn()?;

        if let Some(entry_bytes) = self.db_files.get(&rtxn, &id.to_be_bytes())? {
            let stored: StoredFileEntry = serde_json::from_slice(entry_bytes)?;
            Ok(Some(stored.into()))
        } else {
            Ok(None)
        }
    }

    /// Get file ID by parent and name (O(1) lookup via secondary index).
    pub fn get_file_id(&self, parent_id: i64, name: &str) -> Result<Option<i64>> {
        let rtxn = self.env.read_txn()?;
        let index_key = format!("{}:{}", parent_id, name);

        if let Some(value) = self.db_file_index.get(&rtxn, index_key.as_bytes())? {
            let id = i64::from_be_bytes(value.try_into().unwrap_or([0; 8]));
            return Ok(Some(id));
        }
        Ok(None)
    }

    /// Check if a name exists under a parent.
    pub fn name_exists(&self, parent_id: i64, name: &str) -> Result<bool> {
        Ok(self.get_file_id(parent_id, name)?.is_some())
    }

    /// List children of a directory.
    pub fn list_children(&self, parent_id: i64) -> Result<Vec<DirEntry>> {
        let rtxn = self.env.read_txn()?;
        let mut children = Vec::new();

        for result in self.db_files.iter(&rtxn)? {
            let (_, value) = result?;
            let stored: StoredFileEntry = serde_json::from_slice(value)?;
            if stored.parent_id == Some(parent_id) {
                let entry: FileEntry = stored.into();
                children.push(DirEntry::from(&entry));
            }
        }

        children.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(children)
    }

    /// Check whether a directory has any children.
    pub fn has_children(&self, parent_id: i64) -> Result<bool> {
        Ok(!self.list_children(parent_id)?.is_empty())
    }

    /// Create a new file.
    pub fn create_file(
        &self,
        parent_id: i64,
        name: &str,
        content_hash: [u8; 32],
        size: u64,
    ) -> Result<i64> {
        // Check if name already exists
        if self.name_exists(parent_id, name)? {
            return Err(VfsError::AlreadyExists(name.into()));
        }

        let file_id = self.next_id(&self.next_file_id);
        let now = Utc::now();
        let entry = StoredFileEntry {
            id: file_id,
            parent_id: Some(parent_id),
            name: name.to_string(),
            file_type: FileType::File as u8,
            content_hash: Some(content_hash),
            size,
            created_at: now.timestamp(),
            modified_at: now.timestamp(),
        };

        let mut wtxn = self.env.write_txn()?;
        self.db_files.put(
            &mut wtxn,
            &file_id.to_be_bytes(),
            &serde_json::to_vec(&entry)?,
        )?;

        // Update secondary index
        let index_key = format!("{}:{}", parent_id, name);
        self.db_file_index
            .put(&mut wtxn, index_key.as_bytes(), &file_id.to_be_bytes())?;

        // Update path cache
        let parent_path = self.get_path_for_id(parent_id)?;
        let file_path = if parent_path == "/" {
            format!("/{}", name)
        } else {
            format!("{}/{}", parent_path, name)
        };
        self.db_paths
            .put(&mut wtxn, file_path.as_bytes(), &file_id.to_be_bytes())?;

        wtxn.commit()?;

        // Increment content ref count
        self.increment_content_ref(&content_hash)?;

        Ok(file_id)
    }

    /// Update a file's content.
    pub fn update_file(&self, file_id: i64, content_hash: [u8; 32], size: u64) -> Result<()> {
        let rtxn = self.env.read_txn()?;
        let entry_bytes = self
            .db_files
            .get(&rtxn, &file_id.to_be_bytes())?
            .ok_or_else(|| VfsError::NotFound(file_id.to_string().into()))?;

        let mut stored: StoredFileEntry = serde_json::from_slice(entry_bytes)?;
        let old_hash = stored.content_hash;
        drop(rtxn);

        // Decrement old content ref
        if let Some(old_hash) = old_hash {
            self.decrement_content_ref(&old_hash)?;
        }

        stored.content_hash = Some(content_hash);
        stored.size = size;
        stored.modified_at = Utc::now().timestamp();

        let mut wtxn = self.env.write_txn()?;
        self.db_files.put(
            &mut wtxn,
            &file_id.to_be_bytes(),
            &serde_json::to_vec(&stored)?,
        )?;
        wtxn.commit()?;

        // Increment new content ref
        self.increment_content_ref(&content_hash)?;

        Ok(())
    }

    /// Create a new directory.
    pub fn create_directory(&self, parent_id: i64, name: &str) -> Result<i64> {
        // Check if name already exists
        if self.name_exists(parent_id, name)? {
            return Err(VfsError::AlreadyExists(name.into()));
        }

        let dir_id = self.next_id(&self.next_file_id);
        let now = Utc::now();
        let entry = StoredFileEntry {
            id: dir_id,
            parent_id: Some(parent_id),
            name: name.to_string(),
            file_type: FileType::Directory as u8,
            content_hash: None,
            size: 0,
            created_at: now.timestamp(),
            modified_at: now.timestamp(),
        };

        let mut wtxn = self.env.write_txn()?;
        self.db_files.put(
            &mut wtxn,
            &dir_id.to_be_bytes(),
            &serde_json::to_vec(&entry)?,
        )?;

        // Update secondary index
        let index_key = format!("{}:{}", parent_id, name);
        self.db_file_index
            .put(&mut wtxn, index_key.as_bytes(), &dir_id.to_be_bytes())?;

        // Update path cache
        let parent_path = self.get_path_for_id(parent_id)?;
        let dir_path = if parent_path == "/" {
            format!("/{}", name)
        } else {
            format!("{}/{}", parent_path, name)
        };
        self.db_paths
            .put(&mut wtxn, dir_path.as_bytes(), &dir_id.to_be_bytes())?;

        wtxn.commit()?;

        Ok(dir_id)
    }

    /// Delete a file or directory.
    pub fn delete_entry(&self, id: i64, recursive: bool) -> Result<()> {
        let rtxn = self.env.read_txn()?;
        let entry_bytes = self
            .db_files
            .get(&rtxn, &id.to_be_bytes())?
            .ok_or_else(|| VfsError::NotFound(id.to_string().into()))?;

        let stored: StoredFileEntry = serde_json::from_slice(entry_bytes)?;
        drop(rtxn);

        // If directory, check if empty or recurse
        if stored.file_type == FileType::Directory as u8 {
            let children = self.list_children(id)?;
            if !children.is_empty() {
                if recursive {
                    // Delete all children first
                    for child in children {
                        if let Some(child_id) = self.get_file_id(id, &child.name)? {
                            self.delete_entry(child_id, true)?;
                        }
                    }
                } else {
                    return Err(VfsError::NotEmpty(stored.name.into()));
                }
            }
        }

        // Decrement content ref if file
        if let Some(hash) = stored.content_hash {
            self.decrement_content_ref(&hash)?;
        }

        // Remove from path cache
        let path = self.get_path_for_id(id)?;
        let mut wtxn = self.env.write_txn()?;
        self.db_paths.delete(&mut wtxn, path.as_bytes())?;

        // Remove from secondary index
        if let Some(parent_id) = stored.parent_id {
            let index_key = format!("{}:{}", parent_id, stored.name);
            self.db_file_index.delete(&mut wtxn, index_key.as_bytes())?;
        }

        // Remove file entry
        self.db_files.delete(&mut wtxn, &id.to_be_bytes())?;
        wtxn.commit()?;

        // Remove from search index
        self.remove_from_index(&path)?;

        // Remove tags
        self.remove_all_tags_from_file(id)?;

        // Remove metadata
        self.delete_all_metadata(id)?;

        // Remove versions
        self.delete_all_versions(id)?;

        Ok(())
    }

    /// Move/rename a file or directory.
    pub fn move_entry(&self, id: i64, new_parent_id: i64, new_name: &str) -> Result<()> {
        // Check if target name already exists
        if self.name_exists(new_parent_id, new_name)? {
            return Err(VfsError::AlreadyExists(new_name.into()));
        }

        let rtxn = self.env.read_txn()?;
        let entry_bytes = self
            .db_files
            .get(&rtxn, &id.to_be_bytes())?
            .ok_or_else(|| VfsError::NotFound(id.to_string().into()))?;

        let mut stored: StoredFileEntry = serde_json::from_slice(entry_bytes)?;
        drop(rtxn);

        let old_path = self.get_path_for_id(id)?;
        let old_parent_id = stored.parent_id;
        let old_name = stored.name.clone();

        // Update entry
        stored.parent_id = Some(new_parent_id);
        stored.name = new_name.to_string();
        stored.modified_at = Utc::now().timestamp();

        let mut wtxn = self.env.write_txn()?;
        self.db_files
            .put(&mut wtxn, &id.to_be_bytes(), &serde_json::to_vec(&stored)?)?;

        // Update secondary index
        if let Some(old_parent) = old_parent_id {
            let old_index_key = format!("{}:{}", old_parent, old_name);
            self.db_file_index
                .delete(&mut wtxn, old_index_key.as_bytes())?;
        }
        let new_index_key = format!("{}:{}", new_parent_id, new_name);
        self.db_file_index
            .put(&mut wtxn, new_index_key.as_bytes(), &id.to_be_bytes())?;

        // Update path cache
        self.db_paths.delete(&mut wtxn, old_path.as_bytes())?;
        let parent_path = self.get_path_for_id(new_parent_id)?;
        let new_path = if parent_path == "/" {
            format!("/{}", new_name)
        } else {
            format!("{}/{}", parent_path, new_name)
        };
        self.db_paths
            .put(&mut wtxn, new_path.as_bytes(), &id.to_be_bytes())?;

        wtxn.commit()?;

        // Update search index
        self.remove_from_index(&old_path)?;
        if stored.file_type == FileType::File as u8 {
            if let Some(hash) = stored.content_hash {
                if let Ok(content) = self.read_content(&hash) {
                    if let Ok(text) = String::from_utf8(content) {
                        let _ = self.index_file(&new_path, &text);
                    }
                }
            }
        }

        // If directory, update all child paths recursively
        if stored.file_type == FileType::Directory as u8 {
            self.update_child_paths(id, &old_path, &new_path)?;
        }

        Ok(())
    }

    /// Update child paths after a directory move.
    fn update_child_paths(&self, dir_id: i64, old_prefix: &str, new_prefix: &str) -> Result<()> {
        let children = self.list_children(dir_id)?;
        for child in children {
            if let Some(child_id) = self.get_file_id(dir_id, &child.name)? {
                let old_child_path = format!("{}/{}", old_prefix, child.name);
                let new_child_path = format!("{}/{}", new_prefix, child.name);

                let mut wtxn = self.env.write_txn()?;
                self.db_paths.delete(&mut wtxn, old_child_path.as_bytes())?;
                self.db_paths.put(
                    &mut wtxn,
                    new_child_path.as_bytes(),
                    &child_id.to_be_bytes(),
                )?;
                wtxn.commit()?;

                if child.file_type.is_dir() {
                    self.update_child_paths(child_id, &old_child_path, &new_child_path)?;
                }
            }
        }

        Ok(())
    }

    /// Copy a file.
    pub fn copy_file(&self, source_id: i64, dest_parent_id: i64, dest_name: &str) -> Result<i64> {
        let rtxn = self.env.read_txn()?;
        let entry_bytes = self
            .db_files
            .get(&rtxn, &source_id.to_be_bytes())?
            .ok_or_else(|| VfsError::NotFound(source_id.to_string().into()))?;

        let stored: StoredFileEntry = serde_json::from_slice(entry_bytes)?;
        drop(rtxn);

        if stored.file_type != FileType::File as u8 {
            return Err(VfsError::NotAFile(stored.name.into()));
        }

        let content_hash = stored
            .content_hash
            .ok_or_else(|| VfsError::Internal("File has no content".to_string()))?;

        // Create new file with same content (shares the blob)
        self.create_file(dest_parent_id, dest_name, content_hash, stored.size)
    }

    /// Get path for a file ID.
    fn get_path_for_id(&self, id: i64) -> Result<String> {
        let rtxn = self.env.read_txn()?;

        for result in self.db_paths.iter(&rtxn)? {
            let (key, value) = result?;
            let stored_id = i64::from_be_bytes(value.try_into().unwrap_or([0; 8]));
            if stored_id == id {
                return Ok(String::from_utf8_lossy(key).to_string());
            }
        }

        Err(VfsError::NotFound(id.to_string().into()))
    }

    /// Get all file IDs.
    pub fn get_all_file_ids(&self) -> Result<Vec<i64>> {
        let rtxn = self.env.read_txn()?;
        let mut ids = Vec::new();

        for result in self.db_files.iter(&rtxn)? {
            let (key, _) = result?;
            let id = i64::from_be_bytes(key.try_into().unwrap_or([0; 8]));
            ids.push(id);
        }

        Ok(ids)
    }

    // =========================================================================
    // Content Operations
    // =========================================================================

    /// Write content and return its hash.
    pub fn write_content(&self, data: &[u8]) -> Result<[u8; 32]> {
        // Calculate SHA-256 hash
        let mut hasher = Sha256::new();
        hasher.update(data);
        let hash: [u8; 32] = hasher.finalize().into();

        // Check if content already exists
        let rtxn = self.env.read_txn()?;
        let exists = self.db_contents.get(&rtxn, &hash)?.is_some();
        drop(rtxn);

        if exists {
            return Ok(hash);
        }

        // Store content
        let stored = StoredContent {
            hash,
            data: data.to_vec(),
            size: data.len() as u64,
            ref_count: 0, // Will be incremented by create_file
        };

        let mut wtxn = self.env.write_txn()?;
        self.db_contents
            .put(&mut wtxn, &hash, &serde_json::to_vec(&stored)?)?;
        wtxn.commit()?;

        Ok(hash)
    }

    /// Read content by hash.
    pub fn read_content(&self, hash: &[u8; 32]) -> Result<Vec<u8>> {
        let rtxn = self.env.read_txn()?;

        let content_bytes = self
            .db_contents
            .get(&rtxn, hash)?
            .ok_or_else(|| VfsError::NotFound(hex::encode(hash).into()))?;

        let stored: StoredContent = serde_json::from_slice(content_bytes)?;
        Ok(stored.data)
    }

    /// Increment content reference count.
    fn increment_content_ref(&self, hash: &[u8; 32]) -> Result<()> {
        let rtxn = self.env.read_txn()?;
        if let Some(content_bytes) = self.db_contents.get(&rtxn, hash)? {
            let mut stored: StoredContent = serde_json::from_slice(content_bytes)?;
            drop(rtxn);

            stored.ref_count += 1;
            let mut wtxn = self.env.write_txn()?;
            self.db_contents
                .put(&mut wtxn, hash, &serde_json::to_vec(&stored)?)?;
            wtxn.commit()?;
        }

        Ok(())
    }

    /// Decrement content reference count.
    fn decrement_content_ref(&self, hash: &[u8; 32]) -> Result<()> {
        let rtxn = self.env.read_txn()?;
        if let Some(content_bytes) = self.db_contents.get(&rtxn, hash)? {
            let mut stored: StoredContent = serde_json::from_slice(content_bytes)?;
            drop(rtxn);

            if stored.ref_count > 0 {
                stored.ref_count -= 1;
            }
            let mut wtxn = self.env.write_txn()?;
            self.db_contents
                .put(&mut wtxn, hash, &serde_json::to_vec(&stored)?)?;
            wtxn.commit()?;
        }

        Ok(())
    }

    // =========================================================================
    // Version Operations
    // =========================================================================

    /// Create a new version for a file.
    pub fn create_version(&self, file_id: i64, content_hash: [u8; 32], size: u64) -> Result<i64> {
        // Get next version number
        let version_number = self.get_latest_version_number(file_id)?.unwrap_or(0) + 1;

        let version_id = self.next_id(&self.next_version_id);
        let now = Utc::now();

        let stored = StoredVersion {
            id: version_id,
            file_id,
            version_number,
            content_hash,
            size,
            created_at: now.timestamp(),
        };

        // Key: file_id + version_number for easy lookup
        let key = format!("{}:{}", file_id, version_number);
        let mut wtxn = self.env.write_txn()?;
        self.db_versions
            .put(&mut wtxn, key.as_bytes(), &serde_json::to_vec(&stored)?)?;

        // Also store by ID for direct lookup
        let id_key = format!("id:{}", version_id);
        self.db_versions
            .put(&mut wtxn, id_key.as_bytes(), &serde_json::to_vec(&stored)?)?;

        wtxn.commit()?;

        // Increment content ref
        self.increment_content_ref(&content_hash)?;

        Ok(version_id)
    }

    /// Get all versions for a file.
    pub fn get_file_versions(&self, file_id: i64) -> Result<Vec<FileVersion>> {
        let rtxn = self.env.read_txn()?;
        let mut versions = Vec::new();

        let prefix = format!("{}:", file_id);
        for result in self.db_versions.iter(&rtxn)? {
            let (key, value) = result?;
            let key_str = String::from_utf8_lossy(key);
            if key_str.starts_with(&prefix) && !key_str.starts_with("id:") {
                let stored: StoredVersion = serde_json::from_slice(value)?;
                versions.push(stored.into());
            }
        }

        versions.sort_by_key(|b| std::cmp::Reverse(b.version_number));
        Ok(versions)
    }

    /// Get a specific version.
    pub fn get_version(&self, file_id: i64, version_number: u64) -> Result<Option<FileVersion>> {
        let rtxn = self.env.read_txn()?;

        let key = format!("{}:{}", file_id, version_number);
        if let Some(version_bytes) = self.db_versions.get(&rtxn, key.as_bytes())? {
            let stored: StoredVersion = serde_json::from_slice(version_bytes)?;
            Ok(Some(stored.into()))
        } else {
            Ok(None)
        }
    }

    /// Get version content.
    pub fn get_version_content(&self, file_id: i64, version_number: u64) -> Result<Vec<u8>> {
        let version = self
            .get_version(file_id, version_number)?
            .ok_or_else(|| VfsError::NotFound(format!("version {}", version_number).into()))?;

        self.read_content(&version.content_hash)
    }

    /// Get latest version number for a file.
    pub fn get_latest_version_number(&self, file_id: i64) -> Result<Option<u64>> {
        let versions = self.get_file_versions(file_id)?;
        Ok(versions.first().map(|v| v.version_number))
    }

    /// Delete all versions for a file.
    fn delete_all_versions(&self, file_id: i64) -> Result<()> {
        let rtxn = self.env.read_txn()?;

        let prefix = format!("{}:", file_id);
        let mut to_delete = Vec::new();

        for result in self.db_versions.iter(&rtxn)? {
            let (key, value) = result?;
            let key_str = String::from_utf8_lossy(key);
            if key_str.starts_with(&prefix) {
                let stored: StoredVersion = serde_json::from_slice(value)?;

                // Decrement content ref
                self.decrement_content_ref(&stored.content_hash)?;

                to_delete.push(key.to_vec());

                // Also remove the id-based key
                let id_key = format!("id:{}", stored.id);
                to_delete.push(id_key.into_bytes());
            }
        }
        drop(rtxn);

        let mut wtxn = self.env.write_txn()?;
        for key in to_delete {
            self.db_versions.delete(&mut wtxn, &key)?;
        }
        wtxn.commit()?;

        Ok(())
    }

    /// Prune versions, keeping only the last N.
    pub fn prune_versions_keep(&self, file_id: i64, keep: u64) -> Result<PruneStats> {
        let versions = self.get_file_versions(file_id)?;

        let mut stats = PruneStats {
            files_processed: 1,
            versions_deleted: 0,
        };

        if versions.len() <= keep as usize {
            return Ok(stats);
        }

        // Keep the first `keep` versions (already sorted by version_number desc)
        for version in versions.iter().skip(keep as usize) {
            let key = format!("{}:{}", file_id, version.version_number);
            let id_key = format!("id:{}", version.id);

            let mut wtxn = self.env.write_txn()?;
            self.db_versions.delete(&mut wtxn, key.as_bytes())?;
            self.db_versions.delete(&mut wtxn, id_key.as_bytes())?;
            wtxn.commit()?;

            self.decrement_content_ref(&version.content_hash)?;

            stats.versions_deleted += 1;
        }

        Ok(stats)
    }

    /// Prune versions older than a timestamp.
    pub fn prune_versions_older_than(
        &self,
        file_id: i64,
        before: DateTime<Utc>,
    ) -> Result<PruneStats> {
        let versions = self.get_file_versions(file_id)?;

        let mut stats = PruneStats {
            files_processed: 1,
            versions_deleted: 0,
        };

        // Always keep at least the latest version
        for version in versions.iter().skip(1) {
            if version.created_at < before {
                let key = format!("{}:{}", file_id, version.version_number);
                let id_key = format!("id:{}", version.id);

                let mut wtxn = self.env.write_txn()?;
                self.db_versions.delete(&mut wtxn, key.as_bytes())?;
                self.db_versions.delete(&mut wtxn, id_key.as_bytes())?;
                wtxn.commit()?;

                self.decrement_content_ref(&version.content_hash)?;

                stats.versions_deleted += 1;
            }
        }

        Ok(stats)
    }

    // =========================================================================
    // Search Operations (Tantivy) - Deferred Indexing
    // =========================================================================

    /// Queue an index operation for deferred batch commit.
    /// This is much faster than immediate commits for write-heavy workloads.
    fn queue_index_update(&self, path: &str, content: Option<&str>) {
        let mut pending = self.pending_index_ops.lock().unwrap();
        match content {
            Some(c) => pending.push(IndexOp::Add {
                path: path.to_string(),
                content: c.to_string(),
            }),
            None => pending.push(IndexOp::Remove {
                path: path.to_string(),
            }),
        }

        // Auto-flush if buffer exceeds threshold
        if pending.len() >= 100 {
            drop(pending);
            let _ = self.flush_index_updates();
        }
    }

    /// Flush all pending index operations in a single batch commit.
    pub fn flush_index_updates(&self) -> Result<()> {
        let ops: Vec<_> = {
            let mut pending = self.pending_index_ops.lock().unwrap();
            std::mem::take(&mut *pending)
        };

        if ops.is_empty() {
            return Ok(());
        }

        let mut writer = self.index_writer.write().unwrap();
        for op in ops {
            match op {
                IndexOp::Add { path, content } => {
                    let term = tantivy::Term::from_field_text(self.field_path, &path);
                    writer.delete_term(term);
                    writer.add_document(doc!(
                        self.field_path => path,
                        self.field_content => content
                    ))?;
                }
                IndexOp::Remove { path } => {
                    let term = tantivy::Term::from_field_text(self.field_path, &path);
                    writer.delete_term(term);
                }
            }
        }
        writer.commit()?;
        Ok(())
    }

    /// Index a file's content for full-text search (deferred).
    pub fn index_file(&self, path: &str, content: &str) -> Result<()> {
        self.queue_index_update(path, Some(content));
        Ok(())
    }

    /// Remove a file from the search index (deferred).
    pub fn remove_from_index(&self, path: &str) -> Result<()> {
        self.queue_index_update(path, None);
        Ok(())
    }

    /// Search for files containing the given query.
    pub fn search_content(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        let searcher = self.index_reader.searcher();
        let query_parser = QueryParser::for_index(&self.index, vec![self.field_content]);

        let parsed_query = query_parser
            .parse_query(query)
            .map_err(|e| VfsError::Internal(e.to_string()))?;

        let top_docs = searcher
            .search(&parsed_query, &TopDocs::with_limit(limit))
            .map_err(|e| VfsError::Internal(e.to_string()))?;

        let mut results = Vec::new();
        for (score, doc_address) in top_docs {
            if let Ok(doc) = searcher.doc::<tantivy::TantivyDocument>(doc_address) {
                if let Some(path_value) = doc.get_first(self.field_path) {
                    if let Some(path) = path_value.as_str() {
                        // Get file_id from path
                        if let Ok(entry) = self.get_entry_by_path(path) {
                            results.push(SearchResult {
                                file_id: entry.id,
                                path: path.to_string(),
                                snippet: String::new(),
                                rank: score as f64,
                            });
                        }
                    }
                }
            }
        }

        Ok(results)
    }

    /// Rebuild the search index from all files.
    pub fn rebuild_search_index(&self) -> Result<u64> {
        // Clear any pending index ops first
        {
            let mut pending = self.pending_index_ops.lock().unwrap();
            pending.clear();
        }

        let mut writer = self.index_writer.write().unwrap();

        // Clear existing index
        writer.delete_all_documents()?;
        writer.commit()?;

        let mut indexed = 0u64;

        // Re-index all files
        let rtxn = self.env.read_txn()?;
        for result in self.db_files.iter(&rtxn)? {
            let (_, value) = result?;
            let stored: StoredFileEntry = serde_json::from_slice(value)?;

            if stored.file_type == FileType::File as u8 {
                if let Some(hash) = stored.content_hash {
                    if let Ok(content) = self.read_content(&hash) {
                        if let Ok(text) = String::from_utf8(content) {
                            let path = self.get_path_for_id(stored.id)?;
                            writer.add_document(doc!(
                                self.field_path => path,
                                self.field_content => text
                            ))?;
                            indexed += 1;
                        }
                    }
                }
            }
        }

        writer.commit()?;
        Ok(indexed)
    }

    /// Sync all pending operations to disk.
    /// This flushes the deferred index updates and saves counters.
    pub fn sync(&self) -> Result<()> {
        self.flush_index_updates()?;
        self.save_counters()?;
        // LMDB automatically syncs on transaction commit, but we can force it
        self.env.force_sync()?;
        Ok(())
    }

    // =========================================================================
    // Tag Operations
    // =========================================================================

    /// Create a new tag.
    pub fn create_tag(&self, name: &str) -> Result<i64> {
        // Check if tag already exists
        if self.get_tag_by_name(name)?.is_some() {
            return Err(VfsError::AlreadyExists(name.into()));
        }

        let tag_id = self.next_id(&self.next_tag_id);
        let now = Utc::now();

        let stored = StoredTag {
            id: tag_id,
            name: name.to_string(),
            created_at: now.timestamp(),
        };

        let mut wtxn = self.env.write_txn()?;
        self.db_tags.put(
            &mut wtxn,
            &tag_id.to_be_bytes(),
            &serde_json::to_vec(&stored)?,
        )?;

        // Also store by name for quick lookup
        let name_key = format!("name:{}", name);
        self.db_tags
            .put(&mut wtxn, name_key.as_bytes(), &tag_id.to_be_bytes())?;

        wtxn.commit()?;

        Ok(tag_id)
    }

    /// Get tag by name.
    pub fn get_tag_by_name(&self, name: &str) -> Result<Option<Tag>> {
        let rtxn = self.env.read_txn()?;

        let name_key = format!("name:{}", name);
        if let Some(id_bytes) = self.db_tags.get(&rtxn, name_key.as_bytes())? {
            let tag_id = i64::from_be_bytes(id_bytes.try_into().unwrap_or([0; 8]));
            if let Some(tag_bytes) = self.db_tags.get(&rtxn, &tag_id.to_be_bytes())? {
                let stored: StoredTag = serde_json::from_slice(tag_bytes)?;
                return Ok(Some(stored.into()));
            }
        }

        Ok(None)
    }

    /// Get or create a tag.
    pub fn get_or_create_tag(&self, name: &str) -> Result<i64> {
        if let Some(tag) = self.get_tag_by_name(name)? {
            Ok(tag.id)
        } else {
            self.create_tag(name)
        }
    }

    /// Delete a tag.
    pub fn delete_tag(&self, tag_id: i64) -> Result<()> {
        let rtxn = self.env.read_txn()?;

        // Get tag name for index removal
        let mut name_key_to_delete = None;
        if let Some(tag_bytes) = self.db_tags.get(&rtxn, &tag_id.to_be_bytes())? {
            let stored: StoredTag = serde_json::from_slice(tag_bytes)?;
            name_key_to_delete = Some(format!("name:{}", stored.name));
        }

        // Collect file-tag associations to delete
        let prefix = format!("tag:{}:", tag_id);
        let mut to_delete = Vec::new();
        for result in self.db_file_tags.iter(&rtxn)? {
            let (key, _) = result?;
            let key_str = String::from_utf8_lossy(key);
            if key_str.starts_with(&prefix) {
                to_delete.push(key.to_vec());
            }
        }
        drop(rtxn);

        let mut wtxn = self.env.write_txn()?;

        // Remove name index
        if let Some(name_key) = name_key_to_delete {
            self.db_tags.delete(&mut wtxn, name_key.as_bytes())?;
        }

        // Remove tag entry
        self.db_tags.delete(&mut wtxn, &tag_id.to_be_bytes())?;

        // Remove all file-tag associations
        for key in to_delete {
            self.db_file_tags.delete(&mut wtxn, &key)?;
        }

        wtxn.commit()?;
        Ok(())
    }

    /// Rename a tag.
    pub fn rename_tag(&self, tag_id: i64, new_name: &str) -> Result<()> {
        // Check if new name already exists
        if self.get_tag_by_name(new_name)?.is_some() {
            return Err(VfsError::AlreadyExists(new_name.into()));
        }

        let rtxn = self.env.read_txn()?;
        if let Some(tag_bytes) = self.db_tags.get(&rtxn, &tag_id.to_be_bytes())? {
            let mut stored: StoredTag = serde_json::from_slice(tag_bytes)?;
            let old_name_key = format!("name:{}", stored.name);
            drop(rtxn);

            // Update tag
            stored.name = new_name.to_string();

            let mut wtxn = self.env.write_txn()?;

            // Remove old name index
            self.db_tags.delete(&mut wtxn, old_name_key.as_bytes())?;

            // Update tag entry
            self.db_tags.put(
                &mut wtxn,
                &tag_id.to_be_bytes(),
                &serde_json::to_vec(&stored)?,
            )?;

            // Add new name index
            let new_name_key = format!("name:{}", new_name);
            self.db_tags
                .put(&mut wtxn, new_name_key.as_bytes(), &tag_id.to_be_bytes())?;

            wtxn.commit()?;
        }

        Ok(())
    }

    /// List all tags.
    pub fn list_tags(&self) -> Result<Vec<Tag>> {
        let rtxn = self.env.read_txn()?;
        let mut tags = Vec::new();

        for result in self.db_tags.iter(&rtxn)? {
            let (key, value) = result?;
            // Skip name index entries
            if key.starts_with(b"name:") {
                continue;
            }
            let stored: StoredTag = serde_json::from_slice(value)?;
            tags.push(stored.into());
        }

        tags.sort_by(|a: &Tag, b: &Tag| a.name.cmp(&b.name));
        Ok(tags)
    }

    /// Add a tag to a file.
    pub fn add_tag_to_file(&self, file_id: i64, tag_id: i64) -> Result<()> {
        let key = format!("file:{}:{}", file_id, tag_id);
        let now = Utc::now().timestamp();

        let mut wtxn = self.env.write_txn()?;
        self.db_file_tags
            .put(&mut wtxn, key.as_bytes(), &now.to_be_bytes())?;

        // Also store reverse index for tag lookups
        let rev_key = format!("tag:{}:{}", tag_id, file_id);
        self.db_file_tags
            .put(&mut wtxn, rev_key.as_bytes(), &now.to_be_bytes())?;

        wtxn.commit()?;
        Ok(())
    }

    /// Remove a tag from a file.
    pub fn remove_tag_from_file(&self, file_id: i64, tag_id: i64) -> Result<()> {
        let key = format!("file:{}:{}", file_id, tag_id);
        let rev_key = format!("tag:{}:{}", tag_id, file_id);

        let mut wtxn = self.env.write_txn()?;
        self.db_file_tags.delete(&mut wtxn, key.as_bytes())?;
        self.db_file_tags.delete(&mut wtxn, rev_key.as_bytes())?;
        wtxn.commit()?;

        Ok(())
    }

    /// Remove all tags from a file.
    fn remove_all_tags_from_file(&self, file_id: i64) -> Result<()> {
        let rtxn = self.env.read_txn()?;

        let prefix = format!("file:{}:", file_id);
        let mut to_delete = Vec::new();

        for result in self.db_file_tags.iter(&rtxn)? {
            let (key, _) = result?;
            let key_str = String::from_utf8_lossy(key);
            if key_str.starts_with(&prefix) {
                // Extract tag_id from key
                if let Some(tag_id_str) = key_str.strip_prefix(&prefix) {
                    if let Ok(tag_id) = tag_id_str.parse::<i64>() {
                        to_delete.push((key.to_vec(), tag_id));
                    }
                }
            }
        }
        drop(rtxn);

        let mut wtxn = self.env.write_txn()?;
        for (key, tag_id) in to_delete {
            self.db_file_tags.delete(&mut wtxn, &key)?;
            let rev_key = format!("tag:{}:{}", tag_id, file_id);
            self.db_file_tags.delete(&mut wtxn, rev_key.as_bytes())?;
        }
        wtxn.commit()?;

        Ok(())
    }

    /// Get tags for a file.
    pub fn get_file_tags(&self, file_id: i64) -> Result<Vec<Tag>> {
        let rtxn = self.env.read_txn()?;

        let prefix = format!("file:{}:", file_id);
        let mut tags = Vec::new();

        for result in self.db_file_tags.iter(&rtxn)? {
            let (key, _) = result?;
            let key_str = String::from_utf8_lossy(key);
            if key_str.starts_with(&prefix) {
                if let Some(tag_id_str) = key_str.strip_prefix(&prefix) {
                    if let Ok(tag_id) = tag_id_str.parse::<i64>() {
                        if let Some(tag_bytes) = self.db_tags.get(&rtxn, &tag_id.to_be_bytes())? {
                            let stored: StoredTag = serde_json::from_slice(tag_bytes)?;
                            tags.push(stored.into());
                        }
                    }
                }
            }
        }

        tags.sort_by(|a: &Tag, b: &Tag| a.name.cmp(&b.name));
        Ok(tags)
    }

    /// Get files with a specific tag.
    pub fn get_files_with_tag(&self, tag_id: i64) -> Result<Vec<i64>> {
        let rtxn = self.env.read_txn()?;

        let prefix = format!("tag:{}:", tag_id);
        let mut file_ids = Vec::new();

        for result in self.db_file_tags.iter(&rtxn)? {
            let (key, _) = result?;
            let key_str = String::from_utf8_lossy(key);
            if key_str.starts_with(&prefix) {
                if let Some(file_id_str) = key_str.strip_prefix(&prefix) {
                    if let Ok(file_id) = file_id_str.parse::<i64>() {
                        file_ids.push(file_id);
                    }
                }
            }
        }

        Ok(file_ids)
    }

    // =========================================================================
    // Metadata Operations
    // =========================================================================

    /// Set metadata on a file.
    pub fn set_metadata(&self, file_id: i64, key: &str, value: &str) -> Result<()> {
        let now = Utc::now().timestamp();
        let meta_key = format!("{}:{}", file_id, key);

        let stored = StoredMetadata {
            file_id,
            key: key.to_string(),
            value: value.to_string(),
            created_at: now,
            modified_at: now,
        };

        let mut wtxn = self.env.write_txn()?;
        self.db_metadata.put(
            &mut wtxn,
            meta_key.as_bytes(),
            &serde_json::to_vec(&stored)?,
        )?;
        wtxn.commit()?;

        Ok(())
    }

    /// Get metadata value.
    pub fn get_metadata(&self, file_id: i64, key: &str) -> Result<Option<String>> {
        let rtxn = self.env.read_txn()?;

        let meta_key = format!("{}:{}", file_id, key);
        if let Some(meta_bytes) = self.db_metadata.get(&rtxn, meta_key.as_bytes())? {
            let stored: StoredMetadata = serde_json::from_slice(meta_bytes)?;
            return Ok(Some(stored.value));
        }

        Ok(None)
    }

    /// Get all metadata for a file.
    pub fn get_all_metadata(&self, file_id: i64) -> Result<Vec<Metadata>> {
        let rtxn = self.env.read_txn()?;

        let prefix = format!("{}:", file_id);
        let mut metadata = Vec::new();

        for result in self.db_metadata.iter(&rtxn)? {
            let (key, value) = result?;
            let key_str = String::from_utf8_lossy(key);
            if key_str.starts_with(&prefix) {
                let stored: StoredMetadata = serde_json::from_slice(value)?;
                metadata.push(Metadata {
                    key: stored.key,
                    value: stored.value,
                    modified_at: Utc.timestamp_opt(stored.modified_at, 0).unwrap(),
                });
            }
        }

        metadata.sort_by(|a, b| a.key.cmp(&b.key));
        Ok(metadata)
    }

    /// Delete metadata.
    pub fn delete_metadata(&self, file_id: i64, key: &str) -> Result<()> {
        let meta_key = format!("{}:{}", file_id, key);

        let mut wtxn = self.env.write_txn()?;
        self.db_metadata.delete(&mut wtxn, meta_key.as_bytes())?;
        wtxn.commit()?;

        Ok(())
    }

    /// Delete all metadata for a file.
    fn delete_all_metadata(&self, file_id: i64) -> Result<()> {
        let rtxn = self.env.read_txn()?;

        let prefix = format!("{}:", file_id);
        let mut to_delete = Vec::new();

        for result in self.db_metadata.iter(&rtxn)? {
            let (key, _) = result?;
            let key_str = String::from_utf8_lossy(key);
            if key_str.starts_with(&prefix) {
                to_delete.push(key.to_vec());
            }
        }
        drop(rtxn);

        let mut wtxn = self.env.write_txn()?;
        for key in to_delete {
            self.db_metadata.delete(&mut wtxn, &key)?;
        }
        wtxn.commit()?;

        Ok(())
    }

    /// Get files with specific metadata.
    pub fn get_files_with_metadata(&self, key: &str, value: Option<&str>) -> Result<Vec<i64>> {
        let rtxn = self.env.read_txn()?;
        let mut file_ids = Vec::new();

        for result in self.db_metadata.iter(&rtxn)? {
            let (_, v) = result?;
            let stored: StoredMetadata = serde_json::from_slice(v)?;
            if stored.key == key {
                if let Some(expected_value) = value {
                    if stored.value == expected_value {
                        file_ids.push(stored.file_id);
                    }
                } else {
                    file_ids.push(stored.file_id);
                }
            }
        }

        Ok(file_ids)
    }

    // =========================================================================
    // Snapshot Operations
    // =========================================================================

    /// Save a snapshot of the current vault state.
    pub fn save_snapshot(&self, name: &str, description: Option<&str>) -> Result<i64> {
        // Check if name already exists
        let name_key = format!("name:{}", name);
        let rtxn = self.env.read_txn()?;
        if self.db_snapshots.get(&rtxn, name_key.as_bytes())?.is_some() {
            return Err(VfsError::AlreadyExists(name.into()));
        }
        drop(rtxn);

        let snapshot_id = self.next_id(&self.next_snapshot_id);
        let now = Utc::now();

        // Collect all files
        let rtxn = self.env.read_txn()?;

        let mut file_count = 0i64;
        let mut total_size = 0i64;
        let mut snapshot_files = Vec::new();

        for result in self.db_files.iter(&rtxn)? {
            let (_, value) = result?;
            let stored: StoredFileEntry = serde_json::from_slice(value)?;

            if stored.id == 1 {
                continue; // Skip root
            }

            // Get path for this file
            let path = self.get_path_for_id(stored.id).unwrap_or_default();

            let snapshot_file = StoredSnapshotFile {
                snapshot_id,
                path,
                file_type: stored.file_type,
                content_hash: stored.content_hash,
                size: stored.size,
                created_at: stored.created_at,
                modified_at: stored.modified_at,
            };

            snapshot_files.push((file_count, snapshot_file));
            file_count += 1;
            total_size += stored.size as i64;
        }
        drop(rtxn);

        // Write snapshot files
        let mut wtxn = self.env.write_txn()?;
        for (idx, sf) in snapshot_files {
            let file_key = format!("{}:{}", snapshot_id, idx);
            self.db_snapshot_files.put(
                &mut wtxn,
                file_key.as_bytes(),
                &serde_json::to_vec(&sf)?,
            )?;
        }

        // Create snapshot entry
        let snapshot = StoredSnapshot {
            id: snapshot_id,
            name: name.to_string(),
            created_at: now.timestamp(),
            file_count,
            total_size,
            description: description.map(|s| s.to_string()),
        };

        self.db_snapshots.put(
            &mut wtxn,
            &snapshot_id.to_be_bytes(),
            &serde_json::to_vec(&snapshot)?,
        )?;
        self.db_snapshots
            .put(&mut wtxn, name_key.as_bytes(), &snapshot_id.to_be_bytes())?;

        wtxn.commit()?;

        Ok(snapshot_id)
    }

    /// List all snapshots.
    pub fn list_snapshots(&self) -> Result<Vec<SnapshotInfo>> {
        let rtxn = self.env.read_txn()?;
        let mut snapshots = Vec::new();

        for result in self.db_snapshots.iter(&rtxn)? {
            let (key, value) = result?;
            // Skip name index entries
            if key.starts_with(b"name:") {
                continue;
            }
            let stored: StoredSnapshot = serde_json::from_slice(value)?;
            snapshots.push(stored.into());
        }

        snapshots.sort_by_key(|b| std::cmp::Reverse(b.created_at));
        Ok(snapshots)
    }

    /// Get snapshot info.
    pub fn get_snapshot_info(&self, name: &str) -> Result<Option<SnapshotInfo>> {
        let rtxn = self.env.read_txn()?;

        let name_key = format!("name:{}", name);
        if let Some(id_bytes) = self.db_snapshots.get(&rtxn, name_key.as_bytes())? {
            let snapshot_id = i64::from_be_bytes(id_bytes.try_into().unwrap_or([0; 8]));
            if let Some(snapshot_bytes) =
                self.db_snapshots.get(&rtxn, &snapshot_id.to_be_bytes())?
            {
                let stored: StoredSnapshot = serde_json::from_slice(snapshot_bytes)?;
                return Ok(Some(stored.into()));
            }
        }

        Ok(None)
    }

    /// Restore from a snapshot.
    pub fn restore_snapshot(&self, name: &str) -> Result<RestoreStats> {
        let snapshot = self
            .get_snapshot_info(name)?
            .ok_or_else(|| VfsError::NotFound(name.into()))?;

        // Clear current filesystem (except root)
        let file_ids = self.get_all_file_ids()?;
        for id in file_ids {
            if id != 1 {
                let _ = self.delete_entry(id, true);
            }
        }

        let mut stats = RestoreStats {
            files_restored: 0,
            dirs_restored: 0,
        };

        // Collect all snapshot files
        let rtxn = self.env.read_txn()?;
        let prefix = format!("{}:", snapshot.id);
        let mut snapshot_files = Vec::new();

        for result in self.db_snapshot_files.iter(&rtxn)? {
            let (key, value) = result?;
            let key_str = String::from_utf8_lossy(key);
            if key_str.starts_with(&prefix) {
                let stored: StoredSnapshotFile = serde_json::from_slice(value)?;
                snapshot_files.push(stored);
            }
        }
        drop(rtxn);

        // Sort by path depth to create directories first
        snapshot_files.sort_by(|a, b| {
            let depth_a = a.path.matches('/').count();
            let depth_b = b.path.matches('/').count();
            depth_a.cmp(&depth_b)
        });

        // Restore files
        for sf in snapshot_files {
            let path = &sf.path;
            let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();

            if parts.is_empty() {
                continue;
            }

            // Ensure parent directories exist
            let mut current_parent_id = 1i64; // root
            for part in parts.iter().take(parts.len() - 1) {
                let dir_name = part;
                if let Some(existing_id) = self.get_file_id(current_parent_id, dir_name)? {
                    current_parent_id = existing_id;
                } else {
                    current_parent_id = self.create_directory(current_parent_id, dir_name)?;
                    stats.dirs_restored += 1;
                }
            }

            let name = parts.last().unwrap();

            if sf.file_type == FileType::Directory as u8 {
                if self.get_file_id(current_parent_id, name)?.is_none() {
                    self.create_directory(current_parent_id, name)?;
                    stats.dirs_restored += 1;
                }
            } else if let Some(hash) = sf.content_hash {
                self.create_file(current_parent_id, name, hash, sf.size)?;
                stats.files_restored += 1;
            }
        }

        Ok(stats)
    }

    /// Delete a snapshot.
    pub fn delete_snapshot(&self, name: &str) -> Result<()> {
        let rtxn = self.env.read_txn()?;
        let name_key = format!("name:{}", name);
        let id_bytes = self
            .db_snapshots
            .get(&rtxn, name_key.as_bytes())?
            .ok_or_else(|| VfsError::NotFound(name.into()))?;

        let snapshot_id = i64::from_be_bytes(id_bytes.try_into().unwrap_or([0; 8]));

        // Collect snapshot files to delete
        let prefix = format!("{}:", snapshot_id);
        let mut to_delete = Vec::new();
        for result in self.db_snapshot_files.iter(&rtxn)? {
            let (key, _) = result?;
            let key_str = String::from_utf8_lossy(key);
            if key_str.starts_with(&prefix) {
                to_delete.push(key.to_vec());
            }
        }
        drop(rtxn);

        let mut wtxn = self.env.write_txn()?;

        // Delete snapshot files
        for key in to_delete {
            self.db_snapshot_files.delete(&mut wtxn, &key)?;
        }

        // Delete snapshot
        self.db_snapshots
            .delete(&mut wtxn, &snapshot_id.to_be_bytes())?;
        self.db_snapshots.delete(&mut wtxn, name_key.as_bytes())?;

        wtxn.commit()?;
        Ok(())
    }

    // =========================================================================
    // Quota Operations
    // =========================================================================

    /// Get a quota setting.
    pub fn get_quota(&self, key: &str) -> Result<Option<u64>> {
        let rtxn = self.env.read_txn()?;

        let quota_key = format!("quota:{}", key);
        if let Some(value_bytes) = self.db_settings.get(&rtxn, quota_key.as_bytes())? {
            let value = u64::from_be_bytes(value_bytes.try_into().unwrap_or([0; 8]));
            return Ok(Some(value));
        }

        Ok(None)
    }

    /// Set a quota setting.
    pub fn set_quota(&self, key: &str, value: u64) -> Result<()> {
        let quota_key = format!("quota:{}", key);

        let mut wtxn = self.env.write_txn()?;
        self.db_settings
            .put(&mut wtxn, quota_key.as_bytes(), &value.to_be_bytes())?;
        wtxn.commit()?;

        Ok(())
    }

    /// Clear a quota setting.
    pub fn clear_quota(&self, key: &str) -> Result<()> {
        let quota_key = format!("quota:{}", key);

        let mut wtxn = self.env.write_txn()?;
        self.db_settings.delete(&mut wtxn, quota_key.as_bytes())?;
        wtxn.commit()?;

        Ok(())
    }

    /// Get all quota settings.
    pub fn get_all_quotas(&self) -> Result<QuotaSettings> {
        Ok(QuotaSettings {
            max_size_mb: self.get_quota("max_size_mb")?,
            max_files: self.get_quota("max_files")?,
            max_file_size_mb: self.get_quota("max_file_size_mb")?,
        })
    }

    /// Check if an operation would exceed quotas.
    pub fn check_quota(&self, additional_size: u64, additional_files: u64) -> Result<QuotaCheck> {
        let stats = self.get_vault_stats()?;
        let quotas = self.get_all_quotas()?;

        // Check max files
        if let Some(max_files) = quotas.max_files {
            let new_files = stats.files + additional_files;
            if new_files > max_files {
                return Ok(QuotaCheck {
                    allowed: false,
                    reason: Some(format!(
                        "Would exceed max files quota ({}/{})",
                        new_files, max_files
                    )),
                    current_size: stats.total_size_bytes,
                    current_files: stats.files,
                    max_size_mb: quotas.max_size_mb,
                    max_files: quotas.max_files,
                    max_file_size_mb: quotas.max_file_size_mb,
                });
            }
        }

        // Check max size
        if let Some(max_size_mb) = quotas.max_size_mb {
            let max_size = max_size_mb * 1024 * 1024;
            let new_size = stats.total_size_bytes + additional_size;
            if new_size > max_size {
                return Ok(QuotaCheck {
                    allowed: false,
                    reason: Some(format!(
                        "Would exceed max size quota ({}/{}MB)",
                        new_size / 1024 / 1024,
                        max_size_mb
                    )),
                    current_size: stats.total_size_bytes,
                    current_files: stats.files,
                    max_size_mb: quotas.max_size_mb,
                    max_files: quotas.max_files,
                    max_file_size_mb: quotas.max_file_size_mb,
                });
            }
        }

        // Check max file size
        if let Some(max_file_size_mb) = quotas.max_file_size_mb {
            let max_file_size = max_file_size_mb * 1024 * 1024;
            if additional_size > max_file_size {
                return Ok(QuotaCheck {
                    allowed: false,
                    reason: Some(format!(
                        "File size exceeds max file size quota ({}MB)",
                        max_file_size_mb
                    )),
                    current_size: stats.total_size_bytes,
                    current_files: stats.files,
                    max_size_mb: quotas.max_size_mb,
                    max_files: quotas.max_files,
                    max_file_size_mb: quotas.max_file_size_mb,
                });
            }
        }

        Ok(QuotaCheck {
            allowed: true,
            reason: None,
            current_size: stats.total_size_bytes,
            current_files: stats.files,
            max_size_mb: quotas.max_size_mb,
            max_files: quotas.max_files,
            max_file_size_mb: quotas.max_file_size_mb,
        })
    }

    // =========================================================================
    // Settings Operations
    // =========================================================================

    /// Get a setting value.
    pub fn get_setting(&self, key: &str) -> Result<Option<String>> {
        let rtxn = self.env.read_txn()?;

        if let Some(value_bytes) = self.db_settings.get(&rtxn, key.as_bytes())? {
            let value = String::from_utf8_lossy(value_bytes).to_string();
            Ok(Some(value))
        } else {
            Ok(None)
        }
    }

    /// Set a setting value.
    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        let mut wtxn = self.env.write_txn()?;
        self.db_settings
            .put(&mut wtxn, key.as_bytes(), value.as_bytes())?;
        wtxn.commit()?;
        Ok(())
    }

    // =========================================================================
    // Audit Log Operations
    // =========================================================================

    /// Log an operation.
    pub fn log_operation(
        &self,
        operation: &str,
        path: Option<&str>,
        details: Option<&str>,
    ) -> Result<()> {
        let audit_id = self.next_id(&self.next_audit_id);
        let now = Utc::now();

        let stored = StoredAuditEntry {
            id: audit_id,
            timestamp: now.timestamp(),
            operation: operation.to_string(),
            path: path.map(|s| s.to_string()),
            details: details.map(|s| s.to_string()),
        };

        let mut wtxn = self.env.write_txn()?;
        self.db_audit.put(
            &mut wtxn,
            &audit_id.to_be_bytes(),
            &serde_json::to_vec(&stored)?,
        )?;
        wtxn.commit()?;

        Ok(())
    }

    /// Get audit log entries.
    pub fn get_audit_log(
        &self,
        limit: Option<usize>,
        since: Option<DateTime<Utc>>,
        operation: Option<&str>,
    ) -> Result<Vec<AuditEntry>> {
        let rtxn = self.env.read_txn()?;
        let mut entries = Vec::new();

        // Collect all entries first (LMDB doesn't have reverse iteration like Sled)
        let mut all_entries: Vec<StoredAuditEntry> = Vec::new();
        for result in self.db_audit.iter(&rtxn)? {
            let (_, value) = result?;
            let stored: StoredAuditEntry = serde_json::from_slice(value)?;
            all_entries.push(stored);
        }

        // Sort by ID descending (newest first)
        all_entries.sort_by_key(|b| std::cmp::Reverse(b.id));

        for stored in all_entries {
            // Apply filters
            if let Some(since_ts) = since {
                let entry_ts = Utc.timestamp_opt(stored.timestamp, 0).unwrap();
                if entry_ts < since_ts {
                    continue;
                }
            }

            if let Some(op) = operation {
                if stored.operation != op {
                    continue;
                }
            }

            entries.push(stored.into());

            if let Some(max) = limit {
                if entries.len() >= max {
                    break;
                }
            }
        }

        Ok(entries)
    }

    /// Clear audit log.
    pub fn clear_audit_log(&self, before: Option<DateTime<Utc>>) -> Result<u64> {
        let rtxn = self.env.read_txn()?;

        let mut to_delete = Vec::new();
        let mut count = 0u64;

        for result in self.db_audit.iter(&rtxn)? {
            let (key, value) = result?;
            let stored: StoredAuditEntry = serde_json::from_slice(value)?;

            let should_delete = if let Some(before_ts) = before {
                let entry_ts = Utc.timestamp_opt(stored.timestamp, 0).unwrap();
                entry_ts < before_ts
            } else {
                true
            };

            if should_delete {
                to_delete.push(key.to_vec());
                count += 1;
            }
        }
        drop(rtxn);

        let mut wtxn = self.env.write_txn()?;
        for key in to_delete {
            self.db_audit.delete(&mut wtxn, &key)?;
        }
        wtxn.commit()?;

        Ok(count)
    }

    /// Get audit log count.
    pub fn get_audit_count(&self) -> Result<u64> {
        let rtxn = self.env.read_txn()?;
        let mut count = 0u64;
        for _ in self.db_audit.iter(&rtxn)? {
            count += 1;
        }
        Ok(count)
    }

    // =========================================================================
    // Maintenance Operations
    // =========================================================================

    /// Get vault statistics.
    pub fn get_vault_stats(&self) -> Result<VaultStats> {
        let rtxn = self.env.read_txn()?;

        let mut files = 0u64;
        let mut directories = 0u64;
        let mut total_size_bytes = 0u64;

        for result in self.db_files.iter(&rtxn)? {
            let (_, value) = result?;
            let stored: StoredFileEntry = serde_json::from_slice(value)?;

            if stored.file_type == FileType::File as u8 {
                files += 1;
                total_size_bytes += stored.size;
            } else {
                directories += 1;
            }
        }

        // Count versions (excluding id: prefix entries)
        let mut total_versions = 0u64;
        for result in self.db_versions.iter(&rtxn)? {
            let (key, _) = result?;
            if !key.starts_with(b"id:") {
                total_versions += 1;
            }
        }

        // Count content blobs and orphaned
        let mut content_blobs = 0u64;
        let mut orphaned_blobs = 0u64;
        let mut orphaned_bytes = 0u64;

        for result in self.db_contents.iter(&rtxn)? {
            let (_, value) = result?;
            let stored: StoredContent = serde_json::from_slice(value)?;
            content_blobs += 1;
            if stored.ref_count == 0 {
                orphaned_blobs += 1;
                orphaned_bytes += stored.size;
            }
        }

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

    /// Get database size in bytes.
    pub fn get_db_size(&self) -> Result<i64> {
        // LMDB stores data in a directory, sum up all files
        let mut total_size = 0i64;
        if self.path.is_dir() {
            for entry in std::fs::read_dir(&self.path)? {
                let entry = entry?;
                if let Ok(metadata) = entry.metadata() {
                    total_size += metadata.len() as i64;
                }
            }
        }
        Ok(total_size)
    }

    /// Find orphaned content blobs.
    pub fn find_orphaned_blobs(&self) -> Result<Vec<OrphanedBlob>> {
        let rtxn = self.env.read_txn()?;
        let mut orphaned = Vec::new();

        for result in self.db_contents.iter(&rtxn)? {
            let (_, value) = result?;
            let stored: StoredContent = serde_json::from_slice(value)?;

            if stored.ref_count == 0 {
                orphaned.push(OrphanedBlob {
                    hash: stored.hash,
                    size: stored.size,
                });
            }
        }

        Ok(orphaned)
    }

    /// Delete orphaned content blobs.
    pub fn delete_orphaned_blobs(&self) -> Result<GcStats> {
        let rtxn = self.env.read_txn()?;

        let mut stats = GcStats {
            orphans_found: 0,
            orphans_deleted: 0,
            bytes_freed: 0,
        };

        let mut to_delete = Vec::new();

        for result in self.db_contents.iter(&rtxn)? {
            let (key, value) = result?;
            let stored: StoredContent = serde_json::from_slice(value)?;

            if stored.ref_count == 0 {
                stats.orphans_found += 1;
                to_delete.push((key.to_vec(), stored.size));
            }
        }
        drop(rtxn);

        let mut wtxn = self.env.write_txn()?;
        for (key, size) in to_delete {
            self.db_contents.delete(&mut wtxn, &key)?;
            stats.orphans_deleted += 1;
            stats.bytes_freed += size;
        }
        wtxn.commit()?;

        Ok(stats)
    }

    /// Compact the database (LMDB doesn't need explicit compaction, but flush).
    pub fn compact(&self) -> Result<()> {
        // LMDB doesn't have a compact operation like SQLite's VACUUM
        // Just force a sync
        self.env.force_sync()?;
        Ok(())
    }

    /// Recalculate all content reference counts.
    pub fn recalculate_ref_counts(&self) -> Result<()> {
        let rtxn = self.env.read_txn()?;

        // Count references from files
        let mut ref_counts: HashMap<[u8; 32], u32> = HashMap::new();

        for result in self.db_files.iter(&rtxn)? {
            let (_, value) = result?;
            let stored: StoredFileEntry = serde_json::from_slice(value)?;
            if let Some(hash) = stored.content_hash {
                *ref_counts.entry(hash).or_insert(0) += 1;
            }
        }

        // Count references from versions
        for result in self.db_versions.iter(&rtxn)? {
            let (key, value) = result?;
            if key.starts_with(b"id:") {
                continue;
            }
            let stored: StoredVersion = serde_json::from_slice(value)?;
            *ref_counts.entry(stored.content_hash).or_insert(0) += 1;
        }

        // Collect content updates
        let mut updates = Vec::new();
        for result in self.db_contents.iter(&rtxn)? {
            let (key, value) = result?;
            let mut stored: StoredContent = serde_json::from_slice(value)?;
            let new_count = ref_counts.get(&stored.hash).copied().unwrap_or(0);
            if stored.ref_count != new_count {
                stored.ref_count = new_count;
                updates.push((key.to_vec(), stored));
            }
        }
        drop(rtxn);

        // Apply updates
        let mut wtxn = self.env.write_txn()?;
        for (key, stored) in updates {
            self.db_contents
                .put(&mut wtxn, &key, &serde_json::to_vec(&stored)?)?;
        }
        wtxn.commit()?;

        Ok(())
    }
}

// =========================================================================
// Drop Implementation - Ensure data is persisted on shutdown
// =========================================================================

impl Drop for LmdbBackend {
    fn drop(&mut self) {
        // Flush any pending index operations
        let _ = self.flush_index_updates();
        // Persist counters
        let _ = self.save_counters();
        // Force sync to disk
        let _ = self.env.force_sync();
    }
}

// =========================================================================
// StorageBackend Trait Implementation
// =========================================================================

impl StorageBackend for LmdbBackend {
    fn get(&self, collection: &str, key: &[u8]) -> Result<Option<Vec<u8>>> {
        let db = self.get_database(collection)?;
        let rtxn = self.env.read_txn()?;
        Ok(db.get(&rtxn, key)?.map(|v| v.to_vec()))
    }

    fn put(&self, collection: &str, key: &[u8], value: &[u8]) -> Result<()> {
        let db = self.get_database(collection)?;
        let mut wtxn = self.env.write_txn()?;
        db.put(&mut wtxn, key, value)?;
        wtxn.commit()?;
        Ok(())
    }

    fn delete(&self, collection: &str, key: &[u8]) -> Result<()> {
        let db = self.get_database(collection)?;
        let mut wtxn = self.env.write_txn()?;
        db.delete(&mut wtxn, key)?;
        wtxn.commit()?;
        Ok(())
    }

    fn exists(&self, collection: &str, key: &[u8]) -> Result<bool> {
        let db = self.get_database(collection)?;
        let rtxn = self.env.read_txn()?;
        Ok(db.get(&rtxn, key)?.is_some())
    }

    fn scan_all(&self, collection: &str) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        let db = self.get_database(collection)?;
        let rtxn = self.env.read_txn()?;
        let mut results = Vec::new();

        for result in db.iter(&rtxn)? {
            let (key, value) = result?;
            results.push((key.to_vec(), value.to_vec()));
        }

        Ok(results)
    }

    fn scan_prefix(&self, collection: &str, prefix: &[u8]) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        let db = self.get_database(collection)?;
        let rtxn = self.env.read_txn()?;
        let mut results = Vec::new();

        // LMDB doesn't have native prefix scan, so we filter manually
        for result in db.iter(&rtxn)? {
            let (key, value) = result?;
            if key.starts_with(prefix) {
                results.push((key.to_vec(), value.to_vec()));
            }
        }

        Ok(results)
    }

    fn sync(&self) -> Result<()> {
        self.env.force_sync()?;
        Ok(())
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_create_and_read_file() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.lmdb");
        let backend = LmdbBackend::open(&db_path).unwrap();

        // Write content
        let content = b"Hello, LMDB!";
        let hash = backend.write_content(content).unwrap();

        // Create file
        let _file_id = backend
            .create_file(1, "test.txt", hash, content.len() as u64)
            .unwrap();

        // Read back
        let entry = backend.get_entry_by_path("/test.txt").unwrap();
        assert_eq!(entry.name, "test.txt");
        assert_eq!(entry.size, content.len() as u64);

        let read_content = backend.read_content(&hash).unwrap();
        assert_eq!(read_content, content);
    }

    #[test]
    fn test_versioning() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.lmdb");
        let backend = LmdbBackend::open(&db_path).unwrap();

        // Create file with initial content
        let content1 = b"Version 1";
        let hash1 = backend.write_content(content1).unwrap();
        let file_id = backend
            .create_file(1, "versioned.txt", hash1, content1.len() as u64)
            .unwrap();

        // Create version 1
        backend
            .create_version(file_id, hash1, content1.len() as u64)
            .unwrap();

        // Update file
        let content2 = b"Version 2";
        let hash2 = backend.write_content(content2).unwrap();
        backend
            .update_file(file_id, hash2, content2.len() as u64)
            .unwrap();

        // Create version 2
        backend
            .create_version(file_id, hash2, content2.len() as u64)
            .unwrap();

        // Check versions
        let versions = backend.get_file_versions(file_id).unwrap();
        assert_eq!(versions.len(), 2);

        // Read version 1
        let v1_content = backend.get_version_content(file_id, 1).unwrap();
        assert_eq!(v1_content, content1);
    }

    #[test]
    fn test_tags() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.lmdb");
        let backend = LmdbBackend::open(&db_path).unwrap();

        // Create a tag
        let tag_id = backend.create_tag("important").unwrap();

        // Create a file
        let hash = backend.write_content(b"content").unwrap();
        let file_id = backend.create_file(1, "tagged.txt", hash, 7).unwrap();

        // Tag the file
        backend.add_tag_to_file(file_id, tag_id).unwrap();

        // Check tags
        let tags = backend.get_file_tags(file_id).unwrap();
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].name, "important");

        // Find files by tag
        let files = backend.get_files_with_tag(tag_id).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0], file_id);
    }

    #[test]
    fn test_storage_backend_trait() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.lmdb");
        let backend = LmdbBackend::open(&db_path).unwrap();

        // Test put/get (use "settings" collection which is predefined)
        backend.put("settings", b"key1", b"value1").unwrap();
        let value = backend.get("settings", b"key1").unwrap();
        assert_eq!(value, Some(b"value1".to_vec()));

        // Test exists
        assert!(backend.exists("settings", b"key1").unwrap());
        assert!(!backend.exists("settings", b"key2").unwrap());

        // Test delete
        backend.delete("settings", b"key1").unwrap();
        assert!(!backend.exists("settings", b"key1").unwrap());
    }
}
