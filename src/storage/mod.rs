//! Storage backend abstraction layer.
//!
//! This module provides a trait-based abstraction over different storage backends
//! (SQLite, Sled, LMDB, etc.). Each backend implements its native concurrency model.

mod blobstore;
mod sqlite;

#[cfg(feature = "sled-backend")]
mod sled;

#[cfg(feature = "lmdb-backend")]
mod lmdb;

pub use blobstore::BlobStore;
pub use sqlite::{
    AuditEntry, GcStats, OrphanedBlob, PruneStats, QuotaCheck, QuotaSettings, RestoreStats,
    SnapshotInfo, SqliteBackend, VaultStats,
};

#[cfg(feature = "sled-backend")]
pub use self::sled::SledBackend;

#[cfg(feature = "lmdb-backend")]
pub use self::lmdb::LmdbBackend;

use std::fmt;
use std::path::Path;
use std::str::FromStr;

use chrono::{TimeZone, Utc};

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
        #[allow(unused_mut)]
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

/// How a vault stores content blobs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StorageMode {
    /// Blobs live inside the vault's own database file (default; legacy behavior).
    #[default]
    SingleFile,
    /// Blobs live in a shared `blobs.avfs` store, so `fork` copies only metadata
    /// (SQLite backend only).
    SharedBlob,
}

impl StorageMode {
    /// The value persisted in the vault's `storage_mode` setting.
    pub fn as_setting(&self) -> &'static str {
        match self {
            StorageMode::SingleFile => "single-file",
            StorageMode::SharedBlob => "shared-blob",
        }
    }

    /// Parse the persisted `storage_mode` setting (absent/unknown ⇒ single-file).
    pub fn from_setting(s: Option<&str>) -> Self {
        match s {
            Some("shared-blob") => StorageMode::SharedBlob,
            _ => StorageMode::SingleFile,
        }
    }
}

use crate::error::Result;
#[cfg(any(feature = "sled-backend", feature = "lmdb-backend"))]
use crate::fs::path as vfs_path;
use crate::fs::{FileEntry, FileVersion, Metadata, SearchResult, Tag};

/// Runtime-selected storage backend wrapper.
pub enum VaultBackend {
    Sqlite(SqliteBackend),
    #[cfg(feature = "sled-backend")]
    Sled(SledBackend),
    #[cfg(feature = "lmdb-backend")]
    Lmdb(LmdbBackend),
}

impl VaultBackend {
    /// Open a vault with the specified backend type. Storage mode is auto-detected from the
    /// vault's persisted `storage_mode` setting (defaults to single-file for legacy vaults).
    pub fn open(path: &Path, backend: BackendType) -> Result<Self> {
        match backend {
            BackendType::Sqlite => Ok(Self::Sqlite(SqliteBackend::open(path)?)),
            #[cfg(feature = "sled-backend")]
            BackendType::Sled => Ok(Self::Sled(SledBackend::open(path)?)),
            #[cfg(feature = "lmdb-backend")]
            BackendType::Lmdb => Ok(Self::Lmdb(LmdbBackend::open(path)?)),
        }
    }

    /// Create/open a vault with an explicit storage mode. Only the SQLite backend supports
    /// `StorageMode::SharedBlob`; other backends ignore the mode and stay single-file.
    pub fn open_with_mode(path: &Path, backend: BackendType, mode: StorageMode) -> Result<Self> {
        match backend {
            BackendType::Sqlite => Ok(Self::Sqlite(SqliteBackend::open_with_mode(path, mode)?)),
            #[cfg(feature = "sled-backend")]
            BackendType::Sled => Ok(Self::Sled(SledBackend::open(path)?)),
            #[cfg(feature = "lmdb-backend")]
            BackendType::Lmdb => Ok(Self::Lmdb(LmdbBackend::open(path)?)),
        }
    }

    /// This vault's storage mode (non-SQLite backends are always single-file).
    pub fn storage_mode(&self) -> StorageMode {
        match self {
            Self::Sqlite(backend) => backend.storage_mode(),
            #[cfg(feature = "sled-backend")]
            Self::Sled(_) => StorageMode::SingleFile,
            #[cfg(feature = "lmdb-backend")]
            Self::Lmdb(_) => StorageMode::SingleFile,
        }
    }

    /// Content hashes referenced by this vault's metadata (for store-wide shared-blob GC).
    pub fn referenced_content_hashes(&self) -> Result<Vec<Vec<u8>>> {
        match self {
            Self::Sqlite(backend) => backend.referenced_content_hashes(),
            #[cfg(feature = "sled-backend")]
            Self::Sled(_) => Ok(Vec::new()),
            #[cfg(feature = "lmdb-backend")]
            Self::Lmdb(_) => Ok(Vec::new()),
        }
    }

    pub fn get_entry_by_path(&self, path: &str) -> Result<FileEntry> {
        match self {
            Self::Sqlite(backend) => backend.get_entry_by_path(path),
            #[cfg(feature = "sled-backend")]
            Self::Sled(backend) => backend.get_entry_by_path(path),
            #[cfg(feature = "lmdb-backend")]
            Self::Lmdb(backend) => backend.get_entry_by_path(path),
        }
    }

    pub fn get_entry_by_id(&self, id: i64) -> Result<FileEntry> {
        match self {
            Self::Sqlite(backend) => backend.get_entry_by_id(id),
            #[cfg(feature = "sled-backend")]
            Self::Sled(backend) => backend.get_entry_by_id(id)?.ok_or_else(|| {
                crate::error::VfsError::Internal(format!("file entry not found: id={id}"))
            }),
            #[cfg(feature = "lmdb-backend")]
            Self::Lmdb(backend) => backend.get_entry_by_id(id)?.ok_or_else(|| {
                crate::error::VfsError::Internal(format!("file entry not found: id={id}"))
            }),
        }
    }

    pub fn list_children(&self, parent_path: &str) -> Result<Vec<FileEntry>> {
        let parent = self.get_entry_by_path(parent_path)?;

        let mut children = match self {
            Self::Sqlite(backend) => backend.list_children(parent.id)?,
            #[cfg(feature = "sled-backend")]
            Self::Sled(backend) => {
                let children = backend.list_children(parent.id)?;
                let mut entries = Vec::with_capacity(children.len());
                for child in children {
                    let child_path = vfs_path::join(parent_path, &child.name)?;
                    entries.push(backend.get_entry_by_path(&child_path)?);
                }
                entries
            }
            #[cfg(feature = "lmdb-backend")]
            Self::Lmdb(backend) => {
                let children = backend.list_children(parent.id)?;
                let mut entries = Vec::with_capacity(children.len());
                for child in children {
                    let child_path = vfs_path::join(parent_path, &child.name)?;
                    entries.push(backend.get_entry_by_path(&child_path)?);
                }
                entries
            }
        };

        children.sort_by(|a, b| {
            b.file_type
                .to_i64()
                .cmp(&a.file_type.to_i64())
                .then_with(|| a.name.cmp(&b.name))
        });

        Ok(children)
    }

    pub fn read_content(&self, hash: &[u8; 32]) -> Result<Vec<u8>> {
        match self {
            Self::Sqlite(backend) => backend.read_content(hash),
            #[cfg(feature = "sled-backend")]
            Self::Sled(backend) => backend.read_content(hash),
            #[cfg(feature = "lmdb-backend")]
            Self::Lmdb(backend) => backend.read_content(hash),
        }
    }

    pub fn write_content(&self, data: &[u8]) -> Result<[u8; 32]> {
        match self {
            Self::Sqlite(backend) => backend.write_content(data),
            #[cfg(feature = "sled-backend")]
            Self::Sled(backend) => backend.write_content(data),
            #[cfg(feature = "lmdb-backend")]
            Self::Lmdb(backend) => backend.write_content(data),
        }
    }

    pub fn create_file(
        &self,
        parent_id: i64,
        name: &str,
        content_hash: &[u8; 32],
        size: u64,
        path: &str,
    ) -> Result<i64> {
        match self {
            Self::Sqlite(backend) => backend.create_file(parent_id, name, content_hash, size, path),
            #[cfg(feature = "sled-backend")]
            Self::Sled(backend) => backend.create_file(parent_id, name, *content_hash, size),
            #[cfg(feature = "lmdb-backend")]
            Self::Lmdb(backend) => backend.create_file(parent_id, name, *content_hash, size),
        }
    }

    pub fn update_file(&self, file_id: i64, content_hash: &[u8; 32], size: u64) -> Result<()> {
        match self {
            Self::Sqlite(backend) => backend.update_file(file_id, content_hash, size),
            #[cfg(feature = "sled-backend")]
            Self::Sled(backend) => backend.update_file(file_id, *content_hash, size),
            #[cfg(feature = "lmdb-backend")]
            Self::Lmdb(backend) => backend.update_file(file_id, *content_hash, size),
        }
    }

    pub fn create_directory(&self, parent_id: i64, name: &str, path: &str) -> Result<i64> {
        match self {
            Self::Sqlite(backend) => backend.create_directory(parent_id, name, path),
            #[cfg(feature = "sled-backend")]
            Self::Sled(backend) => backend.create_directory(parent_id, name),
            #[cfg(feature = "lmdb-backend")]
            Self::Lmdb(backend) => backend.create_directory(parent_id, name),
        }
    }

    pub fn name_exists(&self, parent_id: i64, name: &str) -> Result<bool> {
        match self {
            Self::Sqlite(backend) => backend.name_exists(parent_id, name),
            #[cfg(feature = "sled-backend")]
            Self::Sled(backend) => backend.name_exists(parent_id, name),
            #[cfg(feature = "lmdb-backend")]
            Self::Lmdb(backend) => backend.name_exists(parent_id, name),
        }
    }

    pub fn get_file_id(&self, parent_id: i64, name: &str) -> Result<Option<i64>> {
        match self {
            Self::Sqlite(backend) => backend.get_file_id(parent_id, name),
            #[cfg(feature = "sled-backend")]
            Self::Sled(backend) => backend.get_file_id(parent_id, name),
            #[cfg(feature = "lmdb-backend")]
            Self::Lmdb(backend) => backend.get_file_id(parent_id, name),
        }
    }

    pub fn delete_entry(&self, id: i64, path: &str, recursive: bool) -> Result<()> {
        let _ = recursive;
        match self {
            Self::Sqlite(backend) => backend.delete_entry(id, path),
            #[cfg(feature = "sled-backend")]
            Self::Sled(backend) => backend.delete_entry(id, recursive),
            #[cfg(feature = "lmdb-backend")]
            Self::Lmdb(backend) => backend.delete_entry(id, recursive),
        }
    }

    pub fn has_children(&self, id: i64) -> Result<bool> {
        match self {
            Self::Sqlite(backend) => backend.has_children(id),
            #[cfg(feature = "sled-backend")]
            Self::Sled(backend) => backend.has_children(id),
            #[cfg(feature = "lmdb-backend")]
            Self::Lmdb(backend) => backend.has_children(id),
        }
    }

    pub fn move_entry(
        &self,
        id: i64,
        new_parent_id: i64,
        new_name: &str,
        old_path: &str,
        new_path: &str,
    ) -> Result<()> {
        match self {
            Self::Sqlite(backend) => {
                backend.move_entry(id, new_parent_id, new_name, old_path, new_path)
            }
            #[cfg(feature = "sled-backend")]
            Self::Sled(backend) => backend.move_entry(id, new_parent_id, new_name),
            #[cfg(feature = "lmdb-backend")]
            Self::Lmdb(backend) => backend.move_entry(id, new_parent_id, new_name),
        }
    }

    pub fn rebuild_child_paths(&self, parent_id: i64, parent_path: &str) -> Result<()> {
        match self {
            Self::Sqlite(backend) => backend.rebuild_child_paths(parent_id, parent_path),
            #[cfg(feature = "sled-backend")]
            Self::Sled(_) => Ok(()),
            #[cfg(feature = "lmdb-backend")]
            Self::Lmdb(_) => Ok(()),
        }
    }

    pub fn copy_file(
        &self,
        src: &FileEntry,
        new_parent_id: i64,
        new_name: &str,
        new_path: &str,
    ) -> Result<i64> {
        match self {
            Self::Sqlite(backend) => backend.copy_file(src, new_parent_id, new_name, new_path),
            #[cfg(feature = "sled-backend")]
            Self::Sled(backend) => backend.copy_file(src.id, new_parent_id, new_name),
            #[cfg(feature = "lmdb-backend")]
            Self::Lmdb(backend) => backend.copy_file(src.id, new_parent_id, new_name),
        }
    }

    pub fn create_version(&self, file_id: i64, content_hash: &[u8; 32], size: u64) -> Result<u64> {
        match self {
            Self::Sqlite(backend) => backend.create_version(file_id, content_hash, size),
            #[cfg(feature = "sled-backend")]
            Self::Sled(backend) => Ok(backend.create_version(file_id, *content_hash, size)? as u64),
            #[cfg(feature = "lmdb-backend")]
            Self::Lmdb(backend) => Ok(backend.create_version(file_id, *content_hash, size)? as u64),
        }
    }

    pub fn sync_file_index(&self, path: &str) -> Result<()> {
        let entry = self.get_entry_by_path(path)?;
        if !entry.is_file() {
            return Ok(());
        }

        match self {
            Self::Sqlite(backend) => backend.sync_file_index(entry.id, path),
            #[cfg(feature = "sled-backend")]
            Self::Sled(backend) => {
                backend.remove_from_index(path)?;
                if let Some(hash) = entry.content_hash {
                    let data = backend.read_content(&hash)?;
                    if let Ok(content) = String::from_utf8(data) {
                        backend.index_file(path, &content)?;
                    }
                }
                Ok(())
            }
            #[cfg(feature = "lmdb-backend")]
            Self::Lmdb(backend) => {
                backend.remove_from_index(path)?;
                if let Some(hash) = entry.content_hash {
                    let data = backend.read_content(&hash)?;
                    if let Ok(content) = String::from_utf8(data) {
                        backend.index_file(path, &content)?;
                    }
                }
                Ok(())
            }
        }
    }

    pub fn remove_from_index(&self, path: &str) -> Result<()> {
        match self {
            Self::Sqlite(backend) => {
                let entry = backend.get_entry_by_path(path)?;
                backend.remove_from_index(entry.id)
            }
            #[cfg(feature = "sled-backend")]
            Self::Sled(backend) => backend.remove_from_index(path),
            #[cfg(feature = "lmdb-backend")]
            Self::Lmdb(backend) => backend.remove_from_index(path),
        }
    }

    pub fn get_setting(&self, key: &str) -> Result<Option<String>> {
        match self {
            Self::Sqlite(backend) => backend.get_setting(key),
            #[cfg(feature = "sled-backend")]
            Self::Sled(backend) => backend.get_setting(key),
            #[cfg(feature = "lmdb-backend")]
            Self::Lmdb(backend) => backend.get_setting(key),
        }
    }

    pub fn get_file_versions(&self, file_id: i64) -> Result<Vec<FileVersion>> {
        match self {
            Self::Sqlite(backend) => backend.get_file_versions(file_id),
            #[cfg(feature = "sled-backend")]
            Self::Sled(backend) => backend.get_file_versions(file_id),
            #[cfg(feature = "lmdb-backend")]
            Self::Lmdb(backend) => backend.get_file_versions(file_id),
        }
    }

    pub fn get_version_content(&self, file_id: i64, version_num: u64) -> Result<Vec<u8>> {
        match self {
            Self::Sqlite(backend) => backend.get_version_content(file_id, version_num),
            #[cfg(feature = "sled-backend")]
            Self::Sled(backend) => backend.get_version_content(file_id, version_num),
            #[cfg(feature = "lmdb-backend")]
            Self::Lmdb(backend) => backend.get_version_content(file_id, version_num),
        }
    }

    pub fn get_latest_version_number(&self, file_id: i64) -> Result<u64> {
        match self {
            Self::Sqlite(backend) => backend.get_latest_version_number(file_id),
            #[cfg(feature = "sled-backend")]
            Self::Sled(backend) => Ok(backend.get_latest_version_number(file_id)?.unwrap_or(0)),
            #[cfg(feature = "lmdb-backend")]
            Self::Lmdb(backend) => Ok(backend.get_latest_version_number(file_id)?.unwrap_or(0)),
        }
    }

    pub fn rebuild_search_index(&self) -> Result<u64> {
        match self {
            Self::Sqlite(backend) => backend.rebuild_search_index(),
            #[cfg(feature = "sled-backend")]
            Self::Sled(backend) => backend.rebuild_search_index(),
            #[cfg(feature = "lmdb-backend")]
            Self::Lmdb(backend) => backend.rebuild_search_index(),
        }
    }

    pub fn search_content(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        match self {
            Self::Sqlite(backend) => backend.search_content(query, limit),
            #[cfg(feature = "sled-backend")]
            Self::Sled(backend) => backend.search_content(query, limit),
            #[cfg(feature = "lmdb-backend")]
            Self::Lmdb(backend) => backend.search_content(query, limit),
        }
    }

    pub fn create_tag(&self, name: &str) -> Result<i64> {
        match self {
            Self::Sqlite(backend) => backend.create_tag(name),
            #[cfg(feature = "sled-backend")]
            Self::Sled(backend) => backend.create_tag(name),
            #[cfg(feature = "lmdb-backend")]
            Self::Lmdb(backend) => backend.create_tag(name),
        }
    }

    pub fn delete_tag(&self, tag_id: i64) -> Result<()> {
        match self {
            Self::Sqlite(backend) => backend.delete_tag(tag_id),
            #[cfg(feature = "sled-backend")]
            Self::Sled(backend) => backend.delete_tag(tag_id),
            #[cfg(feature = "lmdb-backend")]
            Self::Lmdb(backend) => backend.delete_tag(tag_id),
        }
    }

    pub fn rename_tag(&self, tag_id: i64, new_name: &str) -> Result<()> {
        match self {
            Self::Sqlite(backend) => backend.rename_tag(tag_id, new_name),
            #[cfg(feature = "sled-backend")]
            Self::Sled(backend) => backend.rename_tag(tag_id, new_name),
            #[cfg(feature = "lmdb-backend")]
            Self::Lmdb(backend) => backend.rename_tag(tag_id, new_name),
        }
    }

    pub fn list_tags(&self) -> Result<Vec<Tag>> {
        match self {
            Self::Sqlite(backend) => backend.list_tags(),
            #[cfg(feature = "sled-backend")]
            Self::Sled(backend) => backend.list_tags(),
            #[cfg(feature = "lmdb-backend")]
            Self::Lmdb(backend) => backend.list_tags(),
        }
    }

    pub fn get_tag_by_name(&self, name: &str) -> Result<Option<Tag>> {
        match self {
            Self::Sqlite(backend) => backend.get_tag_by_name(name),
            #[cfg(feature = "sled-backend")]
            Self::Sled(backend) => backend.get_tag_by_name(name),
            #[cfg(feature = "lmdb-backend")]
            Self::Lmdb(backend) => backend.get_tag_by_name(name),
        }
    }

    pub fn add_tag_to_file(&self, file_id: i64, tag_id: i64) -> Result<()> {
        match self {
            Self::Sqlite(backend) => backend.add_tag_to_file(file_id, tag_id),
            #[cfg(feature = "sled-backend")]
            Self::Sled(backend) => backend.add_tag_to_file(file_id, tag_id),
            #[cfg(feature = "lmdb-backend")]
            Self::Lmdb(backend) => backend.add_tag_to_file(file_id, tag_id),
        }
    }

    pub fn remove_tag_from_file(&self, file_id: i64, tag_id: i64) -> Result<()> {
        match self {
            Self::Sqlite(backend) => backend.remove_tag_from_file(file_id, tag_id),
            #[cfg(feature = "sled-backend")]
            Self::Sled(backend) => backend.remove_tag_from_file(file_id, tag_id),
            #[cfg(feature = "lmdb-backend")]
            Self::Lmdb(backend) => backend.remove_tag_from_file(file_id, tag_id),
        }
    }

    pub fn get_file_tags(&self, file_id: i64) -> Result<Vec<Tag>> {
        match self {
            Self::Sqlite(backend) => backend.get_file_tags(file_id),
            #[cfg(feature = "sled-backend")]
            Self::Sled(backend) => backend.get_file_tags(file_id),
            #[cfg(feature = "lmdb-backend")]
            Self::Lmdb(backend) => backend.get_file_tags(file_id),
        }
    }

    pub fn get_files_with_tag(&self, tag_id: i64) -> Result<Vec<i64>> {
        match self {
            Self::Sqlite(backend) => backend.get_files_with_tag(tag_id),
            #[cfg(feature = "sled-backend")]
            Self::Sled(backend) => backend.get_files_with_tag(tag_id),
            #[cfg(feature = "lmdb-backend")]
            Self::Lmdb(backend) => backend.get_files_with_tag(tag_id),
        }
    }

    pub fn get_or_create_tag(&self, name: &str) -> Result<i64> {
        match self {
            Self::Sqlite(backend) => backend.get_or_create_tag(name),
            #[cfg(feature = "sled-backend")]
            Self::Sled(backend) => backend.get_or_create_tag(name),
            #[cfg(feature = "lmdb-backend")]
            Self::Lmdb(backend) => backend.get_or_create_tag(name),
        }
    }

    pub fn set_metadata(&self, file_id: i64, key: &str, value: &str) -> Result<()> {
        match self {
            Self::Sqlite(backend) => backend.set_metadata(file_id, key, value),
            #[cfg(feature = "sled-backend")]
            Self::Sled(backend) => backend.set_metadata(file_id, key, value),
            #[cfg(feature = "lmdb-backend")]
            Self::Lmdb(backend) => backend.set_metadata(file_id, key, value),
        }
    }

    pub fn get_metadata(&self, file_id: i64, key: &str) -> Result<Option<String>> {
        match self {
            Self::Sqlite(backend) => backend.get_metadata(file_id, key),
            #[cfg(feature = "sled-backend")]
            Self::Sled(backend) => backend.get_metadata(file_id, key),
            #[cfg(feature = "lmdb-backend")]
            Self::Lmdb(backend) => backend.get_metadata(file_id, key),
        }
    }

    pub fn get_all_metadata(&self, file_id: i64) -> Result<Vec<Metadata>> {
        match self {
            Self::Sqlite(backend) => backend.get_all_metadata(file_id),
            #[cfg(feature = "sled-backend")]
            Self::Sled(backend) => backend.get_all_metadata(file_id),
            #[cfg(feature = "lmdb-backend")]
            Self::Lmdb(backend) => backend.get_all_metadata(file_id),
        }
    }

    pub fn delete_metadata(&self, file_id: i64, key: &str) -> Result<()> {
        match self {
            Self::Sqlite(backend) => backend.delete_metadata(file_id, key),
            #[cfg(feature = "sled-backend")]
            Self::Sled(backend) => backend.delete_metadata(file_id, key),
            #[cfg(feature = "lmdb-backend")]
            Self::Lmdb(backend) => backend.delete_metadata(file_id, key),
        }
    }

    pub fn get_files_with_metadata(&self, key: &str, value: &str) -> Result<Vec<i64>> {
        match self {
            Self::Sqlite(backend) => backend.get_files_with_metadata(key, value),
            #[cfg(feature = "sled-backend")]
            Self::Sled(backend) => backend.get_files_with_metadata(key, Some(value)),
            #[cfg(feature = "lmdb-backend")]
            Self::Lmdb(backend) => backend.get_files_with_metadata(key, Some(value)),
        }
    }

    pub fn get_vault_stats(&self) -> Result<VaultStats> {
        match self {
            Self::Sqlite(backend) => backend.get_vault_stats(),
            #[cfg(feature = "sled-backend")]
            Self::Sled(backend) => backend.get_vault_stats(),
            #[cfg(feature = "lmdb-backend")]
            Self::Lmdb(backend) => backend.get_vault_stats(),
        }
    }

    pub fn prune_versions_keep(&self, file_id: i64, keep: u64) -> Result<u64> {
        match self {
            Self::Sqlite(backend) => backend.prune_versions_keep(file_id, keep),
            #[cfg(feature = "sled-backend")]
            Self::Sled(backend) => Ok(backend.prune_versions_keep(file_id, keep)?.versions_deleted),
            #[cfg(feature = "lmdb-backend")]
            Self::Lmdb(backend) => Ok(backend.prune_versions_keep(file_id, keep)?.versions_deleted),
        }
    }

    pub fn prune_versions_older_than(&self, file_id: i64, timestamp: i64) -> Result<u64> {
        #[cfg(any(feature = "sled-backend", feature = "lmdb-backend"))]
        let before = timestamp_to_utc(timestamp)?;
        match self {
            Self::Sqlite(backend) => backend.prune_versions_older_than(file_id, timestamp),
            #[cfg(feature = "sled-backend")]
            Self::Sled(backend) => Ok(backend
                .prune_versions_older_than(file_id, before)?
                .versions_deleted),
            #[cfg(feature = "lmdb-backend")]
            Self::Lmdb(backend) => Ok(backend
                .prune_versions_older_than(file_id, before)?
                .versions_deleted),
        }
    }

    pub fn prune_all_versions(
        &self,
        keep: Option<u64>,
        older_than: Option<i64>,
    ) -> Result<PruneStats> {
        match self {
            Self::Sqlite(backend) => backend.prune_all_versions(keep, older_than),
            #[cfg(feature = "sled-backend")]
            Self::Sled(backend) => {
                let mut stats = PruneStats {
                    files_processed: 0,
                    versions_deleted: 0,
                };
                let before = older_than.map(timestamp_to_utc).transpose()?;
                for file_id in backend.get_all_file_ids()? {
                    let pruned = if let Some(keep) = keep {
                        backend.prune_versions_keep(file_id, keep)?
                    } else if let Some(before) = before {
                        backend.prune_versions_older_than(file_id, before)?
                    } else {
                        continue;
                    };
                    stats.files_processed += pruned.files_processed;
                    stats.versions_deleted += pruned.versions_deleted;
                }
                Ok(stats)
            }
            #[cfg(feature = "lmdb-backend")]
            Self::Lmdb(backend) => {
                let mut stats = PruneStats {
                    files_processed: 0,
                    versions_deleted: 0,
                };
                let before = older_than.map(timestamp_to_utc).transpose()?;
                for file_id in backend.get_all_file_ids()? {
                    let pruned = if let Some(keep) = keep {
                        backend.prune_versions_keep(file_id, keep)?
                    } else if let Some(before) = before {
                        backend.prune_versions_older_than(file_id, before)?
                    } else {
                        continue;
                    };
                    stats.files_processed += pruned.files_processed;
                    stats.versions_deleted += pruned.versions_deleted;
                }
                Ok(stats)
            }
        }
    }

    pub fn count_versions_to_prune_keep(&self, file_id: i64, keep: u64) -> Result<u64> {
        match self {
            Self::Sqlite(backend) => backend.count_versions_to_prune_keep(file_id, keep),
            #[cfg(feature = "sled-backend")]
            Self::Sled(backend) => Ok(backend
                .get_file_versions(file_id)?
                .len()
                .saturating_sub(keep as usize) as u64),
            #[cfg(feature = "lmdb-backend")]
            Self::Lmdb(backend) => Ok(backend
                .get_file_versions(file_id)?
                .len()
                .saturating_sub(keep as usize) as u64),
        }
    }

    pub fn count_versions_to_prune_older(&self, file_id: i64, timestamp: i64) -> Result<u64> {
        #[cfg(any(feature = "sled-backend", feature = "lmdb-backend"))]
        let before = timestamp_to_utc(timestamp)?;
        match self {
            Self::Sqlite(backend) => backend.count_versions_to_prune_older(file_id, timestamp),
            #[cfg(feature = "sled-backend")]
            Self::Sled(backend) => Ok(backend
                .get_file_versions(file_id)?
                .into_iter()
                .filter(|version| version.created_at < before)
                .count() as u64),
            #[cfg(feature = "lmdb-backend")]
            Self::Lmdb(backend) => Ok(backend
                .get_file_versions(file_id)?
                .into_iter()
                .filter(|version| version.created_at < before)
                .count() as u64),
        }
    }

    pub fn recalculate_ref_counts(&self) -> Result<()> {
        match self {
            Self::Sqlite(backend) => {
                backend.recalculate_ref_counts()?;
                Ok(())
            }
            #[cfg(feature = "sled-backend")]
            Self::Sled(backend) => backend.recalculate_ref_counts(),
            #[cfg(feature = "lmdb-backend")]
            Self::Lmdb(backend) => backend.recalculate_ref_counts(),
        }
    }

    pub fn find_orphaned_blobs(&self) -> Result<Vec<OrphanedBlob>> {
        match self {
            Self::Sqlite(backend) => backend.find_orphaned_blobs(),
            #[cfg(feature = "sled-backend")]
            Self::Sled(backend) => backend.find_orphaned_blobs(),
            #[cfg(feature = "lmdb-backend")]
            Self::Lmdb(backend) => backend.find_orphaned_blobs(),
        }
    }

    pub fn delete_orphaned_blobs(&self) -> Result<GcStats> {
        match self {
            Self::Sqlite(backend) => backend.delete_orphaned_blobs(),
            #[cfg(feature = "sled-backend")]
            Self::Sled(backend) => backend.delete_orphaned_blobs(),
            #[cfg(feature = "lmdb-backend")]
            Self::Lmdb(backend) => backend.delete_orphaned_blobs(),
        }
    }

    pub fn vacuum(&self) -> Result<()> {
        match self {
            Self::Sqlite(backend) => backend.vacuum(),
            #[cfg(feature = "sled-backend")]
            Self::Sled(backend) => backend.sync(),
            #[cfg(feature = "lmdb-backend")]
            Self::Lmdb(backend) => backend.sync(),
        }
    }

    /// Sync all pending operations to disk.
    /// For Sled/LMDB, this flushes deferred index updates and counters.
    pub fn sync(&self) -> Result<()> {
        match self {
            Self::Sqlite(_) => Ok(()), // SQLite auto-syncs on each operation
            #[cfg(feature = "sled-backend")]
            Self::Sled(backend) => backend.sync(),
            #[cfg(feature = "lmdb-backend")]
            Self::Lmdb(backend) => backend.sync(),
        }
    }

    pub fn get_db_size(&self) -> Result<u64> {
        match self {
            Self::Sqlite(backend) => backend.get_db_size(),
            #[cfg(feature = "sled-backend")]
            Self::Sled(backend) => Ok(backend.get_db_size()? as u64),
            #[cfg(feature = "lmdb-backend")]
            Self::Lmdb(backend) => Ok(backend.get_db_size()? as u64),
        }
    }

    pub fn get_all_file_ids(&self) -> Result<Vec<i64>> {
        match self {
            Self::Sqlite(backend) => backend.get_all_file_ids(),
            #[cfg(feature = "sled-backend")]
            Self::Sled(backend) => backend.get_all_file_ids(),
            #[cfg(feature = "lmdb-backend")]
            Self::Lmdb(backend) => backend.get_all_file_ids(),
        }
    }

    pub fn get_quota(&self, key: &str) -> Result<Option<u64>> {
        match self {
            Self::Sqlite(backend) => backend.get_quota(key),
            #[cfg(feature = "sled-backend")]
            Self::Sled(backend) => backend.get_quota(key),
            #[cfg(feature = "lmdb-backend")]
            Self::Lmdb(backend) => backend.get_quota(key),
        }
    }

    pub fn set_quota(&self, key: &str, value: u64) -> Result<()> {
        match self {
            Self::Sqlite(backend) => backend.set_quota(key, value),
            #[cfg(feature = "sled-backend")]
            Self::Sled(backend) => backend.set_quota(key, value),
            #[cfg(feature = "lmdb-backend")]
            Self::Lmdb(backend) => backend.set_quota(key, value),
        }
    }

    pub fn clear_quota(&self, key: &str) -> Result<()> {
        match self {
            Self::Sqlite(backend) => backend.clear_quota(key),
            #[cfg(feature = "sled-backend")]
            Self::Sled(backend) => backend.clear_quota(key),
            #[cfg(feature = "lmdb-backend")]
            Self::Lmdb(backend) => backend.clear_quota(key),
        }
    }

    pub fn check_quota(&self, new_size: u64, new_file_count: u64) -> Result<QuotaCheck> {
        match self {
            Self::Sqlite(backend) => backend.check_quota(new_size, new_file_count),
            #[cfg(feature = "sled-backend")]
            Self::Sled(backend) => backend.check_quota(new_size, new_file_count),
            #[cfg(feature = "lmdb-backend")]
            Self::Lmdb(backend) => backend.check_quota(new_size, new_file_count),
        }
    }

    pub fn get_all_quotas(&self) -> Result<QuotaSettings> {
        match self {
            Self::Sqlite(backend) => backend.get_all_quotas(),
            #[cfg(feature = "sled-backend")]
            Self::Sled(backend) => backend.get_all_quotas(),
            #[cfg(feature = "lmdb-backend")]
            Self::Lmdb(backend) => backend.get_all_quotas(),
        }
    }

    pub fn log_operation(&self, op: &str, path: Option<&str>, details: Option<&str>) -> Result<()> {
        match self {
            Self::Sqlite(backend) => backend.log_operation(op, path, details),
            #[cfg(feature = "sled-backend")]
            Self::Sled(backend) => backend.log_operation(op, path, details),
            #[cfg(feature = "lmdb-backend")]
            Self::Lmdb(backend) => backend.log_operation(op, path, details),
        }
    }

    pub fn get_audit_log(&self, limit: usize, since: Option<i64>) -> Result<Vec<AuditEntry>> {
        let since = since.map(timestamp_to_utc).transpose()?;
        match self {
            Self::Sqlite(backend) => backend.get_audit_log(limit, since.map(|dt| dt.timestamp())),
            #[cfg(feature = "sled-backend")]
            Self::Sled(backend) => backend.get_audit_log(Some(limit), since, None),
            #[cfg(feature = "lmdb-backend")]
            Self::Lmdb(backend) => backend.get_audit_log(Some(limit), since, None),
        }
    }

    pub fn clear_audit_log(&self, before: Option<i64>) -> Result<u64> {
        let before = before.map(timestamp_to_utc).transpose()?;
        match self {
            Self::Sqlite(backend) => backend.clear_audit_log(before.map(|dt| dt.timestamp())),
            #[cfg(feature = "sled-backend")]
            Self::Sled(backend) => backend.clear_audit_log(before),
            #[cfg(feature = "lmdb-backend")]
            Self::Lmdb(backend) => backend.clear_audit_log(before),
        }
    }

    pub fn get_audit_count(&self) -> Result<u64> {
        match self {
            Self::Sqlite(backend) => backend.get_audit_count(),
            #[cfg(feature = "sled-backend")]
            Self::Sled(backend) => backend.get_audit_count(),
            #[cfg(feature = "lmdb-backend")]
            Self::Lmdb(backend) => backend.get_audit_count(),
        }
    }

    pub fn save_snapshot(&self, name: &str, description: Option<&str>) -> Result<SnapshotInfo> {
        match self {
            Self::Sqlite(backend) => backend.save_snapshot(name, description),
            #[cfg(feature = "sled-backend")]
            Self::Sled(backend) => {
                backend.save_snapshot(name, description)?;
                backend.get_snapshot_info(name)?.ok_or_else(|| {
                    crate::error::VfsError::NotFound(format!("snapshot: {name}").into())
                })
            }
            #[cfg(feature = "lmdb-backend")]
            Self::Lmdb(backend) => {
                backend.save_snapshot(name, description)?;
                backend.get_snapshot_info(name)?.ok_or_else(|| {
                    crate::error::VfsError::NotFound(format!("snapshot: {name}").into())
                })
            }
        }
    }

    pub fn list_snapshots(&self) -> Result<Vec<SnapshotInfo>> {
        match self {
            Self::Sqlite(backend) => backend.list_snapshots(),
            #[cfg(feature = "sled-backend")]
            Self::Sled(backend) => backend.list_snapshots(),
            #[cfg(feature = "lmdb-backend")]
            Self::Lmdb(backend) => backend.list_snapshots(),
        }
    }

    pub fn get_snapshot(&self, name: &str) -> Result<SnapshotInfo> {
        match self {
            Self::Sqlite(backend) => backend.get_snapshot(name),
            #[cfg(feature = "sled-backend")]
            Self::Sled(backend) => backend.get_snapshot_info(name)?.ok_or_else(|| {
                crate::error::VfsError::NotFound(format!("snapshot: {name}").into())
            }),
            #[cfg(feature = "lmdb-backend")]
            Self::Lmdb(backend) => backend.get_snapshot_info(name)?.ok_or_else(|| {
                crate::error::VfsError::NotFound(format!("snapshot: {name}").into())
            }),
        }
    }

    pub fn restore_snapshot(&self, name: &str) -> Result<RestoreStats> {
        match self {
            Self::Sqlite(backend) => backend.restore_snapshot(name),
            #[cfg(feature = "sled-backend")]
            Self::Sled(backend) => backend.restore_snapshot(name),
            #[cfg(feature = "lmdb-backend")]
            Self::Lmdb(backend) => backend.restore_snapshot(name),
        }
    }

    pub fn delete_snapshot(&self, name: &str) -> Result<()> {
        match self {
            Self::Sqlite(backend) => backend.delete_snapshot(name),
            #[cfg(feature = "sled-backend")]
            Self::Sled(backend) => backend.delete_snapshot(name),
            #[cfg(feature = "lmdb-backend")]
            Self::Lmdb(backend) => backend.delete_snapshot(name),
        }
    }
}

fn timestamp_to_utc(timestamp: i64) -> Result<chrono::DateTime<Utc>> {
    Utc.timestamp_opt(timestamp, 0).single().ok_or_else(|| {
        crate::error::VfsError::InvalidInput(format!("invalid timestamp: {timestamp}"))
    })
}

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
