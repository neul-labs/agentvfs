//! Error types for avfs operations.

use std::path::PathBuf;
use thiserror::Error;

/// Result type alias for avfs operations.
pub type Result<T> = std::result::Result<T, VfsError>;

/// Error types for avfs operations.
#[derive(Error, Debug)]
pub enum VfsError {
    /// File or directory not found.
    #[error("not found: {0}")]
    NotFound(PathBuf),

    /// File or directory already exists.
    #[error("already exists: {0}")]
    AlreadyExists(PathBuf),

    /// Expected a directory but found a file.
    #[error("not a directory: {0}")]
    NotADirectory(PathBuf),

    /// Expected a file but found a directory.
    #[error("not a file: {0}")]
    NotAFile(PathBuf),

    /// Directory is not empty (cannot delete).
    #[error("directory not empty: {0}")]
    NotEmpty(PathBuf),

    /// Invalid path format.
    #[error("invalid path: {0}")]
    InvalidPath(String),

    /// Invalid input/arguments.
    #[error("invalid input: {0}")]
    InvalidInput(String),

    /// Vault not found.
    #[error("vault not found: {0}")]
    VaultNotFound(String),

    /// Vault already exists.
    #[error("vault already exists: {0}")]
    VaultExists(String),

    /// No active vault selected.
    #[error("no active vault - run 'avfs vault create <name>' or 'avfs vault use <name>'")]
    NoActiveVault,

    /// Quota exceeded.
    #[error("quota exceeded: {0}")]
    QuotaExceeded(String),

    /// SQLite storage backend error.
    #[error("sqlite error: {0}")]
    Storage(#[from] rusqlite::Error),

    /// Sled storage backend error.
    #[cfg(feature = "sled-backend")]
    #[error("sled error: {0}")]
    Sled(#[from] sled::Error),

    /// LMDB storage backend error.
    #[cfg(feature = "lmdb-backend")]
    #[error("lmdb error: {0}")]
    Lmdb(#[from] heed::Error),

    /// Tantivy search error.
    #[cfg(any(feature = "sled-backend", feature = "lmdb-backend"))]
    #[error("tantivy error: {0}")]
    Tantivy(#[from] tantivy::TantivyError),

    /// IO error.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization error.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    /// External command exited with a non-zero status after already reporting structured output.
    #[error("command exited with code {0}")]
    ExitStatus(i32),

    /// Generic internal error.
    #[error("internal error: {0}")]
    Internal(String),
}

impl VfsError {
    /// Convert error to JSON-serializable format.
    pub fn to_json(&self) -> serde_json::Value {
        let (error_type, message, path) = match self {
            VfsError::NotFound(p) => ("NotFound", self.to_string(), Some(p.display().to_string())),
            VfsError::AlreadyExists(p) => (
                "AlreadyExists",
                self.to_string(),
                Some(p.display().to_string()),
            ),
            VfsError::NotADirectory(p) => (
                "NotADirectory",
                self.to_string(),
                Some(p.display().to_string()),
            ),
            VfsError::NotAFile(p) => ("NotAFile", self.to_string(), Some(p.display().to_string())),
            VfsError::NotEmpty(p) => ("NotEmpty", self.to_string(), Some(p.display().to_string())),
            VfsError::InvalidPath(s) => ("InvalidPath", self.to_string(), Some(s.clone())),
            VfsError::InvalidInput(s) => ("InvalidInput", self.to_string(), Some(s.clone())),
            VfsError::VaultNotFound(s) => ("VaultNotFound", self.to_string(), Some(s.clone())),
            VfsError::VaultExists(s) => ("VaultExists", self.to_string(), Some(s.clone())),
            VfsError::NoActiveVault => ("NoActiveVault", self.to_string(), None),
            VfsError::QuotaExceeded(s) => ("QuotaExceeded", self.to_string(), Some(s.clone())),
            VfsError::Storage(e) => ("StorageError", e.to_string(), None),
            VfsError::Io(e) => ("IoError", e.to_string(), None),
            VfsError::Json(e) => ("JsonError", e.to_string(), None),
            VfsError::ExitStatus(code) => ("ExitStatus", self.to_string(), Some(code.to_string())),
            VfsError::Internal(s) => ("InternalError", s.clone(), None),
            #[cfg(feature = "sled-backend")]
            VfsError::Sled(e) => ("SledError", e.to_string(), None),
            #[cfg(feature = "lmdb-backend")]
            VfsError::Lmdb(e) => ("LmdbError", e.to_string(), None),
            #[cfg(any(feature = "sled-backend", feature = "lmdb-backend"))]
            VfsError::Tantivy(e) => ("TantivyError", e.to_string(), None),
        };

        let mut json = serde_json::json!({
            "error": error_type,
            "message": message,
        });

        if let Some(p) = path {
            json["path"] = serde_json::Value::String(p);
        }

        json
    }
}
