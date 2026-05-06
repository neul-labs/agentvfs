//! File entry and content data structures.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Type of filesystem entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum FileType {
    /// Regular file.
    File = 0,
    /// Directory.
    Directory = 1,
}

impl FileType {
    /// Convert from integer (stored in database).
    pub fn from_i64(val: i64) -> Option<Self> {
        match val {
            0 => Some(FileType::File),
            1 => Some(FileType::Directory),
            _ => None,
        }
    }

    /// Convert to integer for storage.
    pub fn to_i64(self) -> i64 {
        self as i64
    }

    /// Check if this is a directory.
    pub fn is_dir(self) -> bool {
        matches!(self, FileType::Directory)
    }

    /// Check if this is a file.
    pub fn is_file(self) -> bool {
        matches!(self, FileType::File)
    }
}

/// Metadata for a file or directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    /// Unique identifier.
    pub id: i64,
    /// Parent directory ID (None for root).
    pub parent_id: Option<i64>,
    /// File or directory name.
    pub name: String,
    /// Type (file or directory).
    pub file_type: FileType,
    /// SHA-256 hash of content (None for directories).
    pub content_hash: Option<[u8; 32]>,
    /// Size in bytes (0 for directories).
    pub size: u64,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last modification timestamp.
    pub modified_at: DateTime<Utc>,
}

impl FileEntry {
    /// Create a new file entry.
    pub fn new_file(
        id: i64,
        parent_id: i64,
        name: String,
        content_hash: [u8; 32],
        size: u64,
    ) -> Self {
        let now = Utc::now();
        Self {
            id,
            parent_id: Some(parent_id),
            name,
            file_type: FileType::File,
            content_hash: Some(content_hash),
            size,
            created_at: now,
            modified_at: now,
        }
    }

    /// Create a new directory entry.
    pub fn new_dir(id: i64, parent_id: Option<i64>, name: String) -> Self {
        let now = Utc::now();
        Self {
            id,
            parent_id,
            name,
            file_type: FileType::Directory,
            content_hash: None,
            size: 0,
            created_at: now,
            modified_at: now,
        }
    }

    /// Check if this is the root directory.
    pub fn is_root(&self) -> bool {
        self.parent_id.is_none()
    }

    /// Check if this is a directory.
    pub fn is_dir(&self) -> bool {
        self.file_type.is_dir()
    }

    /// Check if this is a file.
    pub fn is_file(&self) -> bool {
        self.file_type.is_file()
    }
}

/// Content blob stored in content-addressable storage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentBlob {
    /// SHA-256 hash of the content.
    pub hash: [u8; 32],
    /// Size in bytes.
    pub size: u64,
    /// Reference count (number of files pointing to this content).
    pub ref_count: u32,
}

impl ContentBlob {
    /// Create a new content blob.
    pub fn new(hash: [u8; 32], size: u64) -> Self {
        Self {
            hash,
            size,
            ref_count: 1,
        }
    }
}

/// Directory listing entry (simplified for ls output).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirEntry {
    /// File or directory name.
    pub name: String,
    /// Type (file or directory).
    pub file_type: FileType,
    /// Size in bytes.
    pub size: u64,
    /// Last modification timestamp.
    pub modified_at: DateTime<Utc>,
}

impl From<&FileEntry> for DirEntry {
    fn from(entry: &FileEntry) -> Self {
        Self {
            name: entry.name.clone(),
            file_type: entry.file_type,
            size: entry.size,
            modified_at: entry.modified_at,
        }
    }
}

/// Version history entry for a file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileVersion {
    /// Unique identifier.
    pub id: i64,
    /// File this version belongs to.
    pub file_id: i64,
    /// Version number (1, 2, 3, ...).
    pub version_number: u64,
    /// SHA-256 hash of the content at this version.
    pub content_hash: [u8; 32],
    /// Size in bytes at this version.
    pub size: u64,
    /// When this version was created.
    pub created_at: DateTime<Utc>,
}

/// Search result from full-text search.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// File ID.
    pub file_id: i64,
    /// File path.
    pub path: String,
    /// Matching snippet with highlights.
    pub snippet: String,
    /// Relevance rank (lower is better).
    pub rank: f64,
}

/// Tag information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    /// Unique identifier.
    pub id: i64,
    /// Tag name.
    pub name: String,
    /// When this tag was created.
    pub created_at: DateTime<Utc>,
}

/// Metadata key-value pair for a file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadata {
    /// Metadata key.
    pub key: String,
    /// Metadata value.
    pub value: String,
    /// When this metadata was last modified.
    pub modified_at: DateTime<Utc>,
}
