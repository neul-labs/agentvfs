//! Vault management operations.

use std::path::PathBuf;
use std::sync::Arc;

use chrono::{DateTime, TimeZone, Utc};

use crate::error::{Result, VfsError};
use crate::runtime::workspace::{dir_size, validate_workspace_name, WorkspaceService};
#[cfg(feature = "lmdb-backend")]
use crate::storage::LmdbBackend;
#[cfg(feature = "sled-backend")]
use crate::storage::SledBackend;
use crate::storage::SqliteBackend;
use crate::storage::{BackendType, StorageMode, VaultBackend};
use crate::vault::Config;

/// Information about a vault.
#[derive(Debug, Clone)]
pub struct VaultInfo {
    /// Vault name.
    pub name: String,
    /// Whether this is the currently active vault.
    pub is_current: bool,
    /// Database file size in bytes.
    pub size: u64,
    /// Creation timestamp.
    pub created_at: Option<DateTime<Utc>>,
    /// Storage backend type.
    pub backend: BackendType,
}

/// Information about a forked vault.
#[derive(Debug, Clone)]
pub struct ForkInfo {
    /// Source vault name.
    pub source: String,
    /// New vault name.
    pub name: String,
    /// Storage backend type.
    pub backend: BackendType,
    /// Backing path of the forked vault.
    pub path: PathBuf,
    /// Whether the fork used copy-on-write cloning.
    pub copy_on_write: bool,
}

/// Vault manager for CRUD operations on vaults.
pub struct VaultManager {
    config: Config,
}

impl VaultManager {
    /// Create a new vault manager.
    pub fn new() -> Result<Self> {
        Ok(Self {
            config: Config::new()?,
        })
    }

    /// Create a vault manager with a custom config (for testing).
    pub fn with_config(config: Config) -> Self {
        Self { config }
    }

    /// Create a new vault with specified backend (single-file storage mode).
    pub fn create_with_backend(&self, name: &str, backend: BackendType) -> Result<()> {
        self.create_with_mode(name, backend, StorageMode::default())
    }

    /// Create a new vault with a specific backend and storage mode.
    ///
    /// `StorageMode::SharedBlob` (SQLite only) routes content to a shared `blobs.avfs` store so
    /// later `fork`s copy only metadata; other backends ignore the mode.
    pub fn create_with_mode(
        &self,
        name: &str,
        backend: BackendType,
        mode: StorageMode,
    ) -> Result<()> {
        validate_workspace_name(name)?;

        if self.config.vault_exists(name) {
            return Err(VfsError::VaultExists(name.to_string()));
        }

        let path = self.config.vault_path_with_backend(name, backend);

        // Create the database based on backend type
        match backend {
            BackendType::Sqlite => {
                let _backend = SqliteBackend::open_with_mode(&path, mode)?;
            }
            #[cfg(feature = "sled-backend")]
            BackendType::Sled => {
                let _backend = SledBackend::open(&path)?;
            }
            #[cfg(feature = "lmdb-backend")]
            BackendType::Lmdb => {
                let _backend = LmdbBackend::open(&path)?;
            }
        }

        // If no current vault, set this as current
        if self.config.current_vault()?.is_none() {
            self.config.set_current_vault(name)?;
        }

        Ok(())
    }

    /// Create a new vault (defaults to SQLite backend).
    pub fn create(&self, name: &str) -> Result<()> {
        self.create_with_backend(name, BackendType::Sqlite)
    }

    /// List all vaults.
    pub fn list(&self) -> Result<Vec<VaultInfo>> {
        let names = self.config.list_vaults()?;

        let mut vaults = Vec::new();
        for name in names {
            let info = self.info(&name)?;
            vaults.push(info);
        }

        Ok(vaults)
    }

    /// Get info about a specific vault.
    pub fn info(&self, name: &str) -> Result<VaultInfo> {
        let backend_type = self
            .config
            .vault_backend(name)
            .ok_or_else(|| VfsError::VaultNotFound(name.to_string()))?;

        let current = self.config.current_vault()?;
        let path = self.config.vault_path_with_backend(name, backend_type);

        let size = if path.is_dir() {
            // For Sled, sum up directory contents
            dir_size(&path)
        } else {
            std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0)
        };

        // Get creation timestamp from the database
        let created_at = match backend_type {
            BackendType::Sqlite => {
                if let Ok(backend) = SqliteBackend::open(&path) {
                    if let Ok(Some(ts_str)) = backend.get_setting("created_at") {
                        if let Ok(ts) = ts_str.parse::<i64>() {
                            Utc.timestamp_opt(ts, 0).single()
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            #[cfg(feature = "sled-backend")]
            BackendType::Sled => {
                if let Ok(backend) = SledBackend::open(&path) {
                    if let Ok(Some(ts_str)) = backend.get_setting("created_at") {
                        if let Ok(ts) = ts_str.parse::<i64>() {
                            Utc.timestamp_opt(ts, 0).single()
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            #[cfg(feature = "lmdb-backend")]
            BackendType::Lmdb => {
                if let Ok(backend) = LmdbBackend::open(&path) {
                    if let Ok(Some(ts_str)) = backend.get_setting("created_at") {
                        if let Ok(ts) = ts_str.parse::<i64>() {
                            Utc.timestamp_opt(ts, 0).single()
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
        };

        Ok(VaultInfo {
            name: name.to_string(),
            is_current: current.as_deref() == Some(name),
            size,
            created_at,
            backend: backend_type,
        })
    }

    /// Switch to a different vault.
    pub fn use_vault(&self, name: &str) -> Result<()> {
        if !self.config.vault_exists(name) {
            return Err(VfsError::VaultNotFound(name.to_string()));
        }

        self.config.set_current_vault(name)?;
        Ok(())
    }

    /// Delete a vault.
    pub fn delete(&self, name: &str) -> Result<()> {
        self.config.delete_vault_file(name)
    }

    /// Fork an existing vault into a new vault.
    pub fn fork(&self, source: &str, name: &str) -> Result<ForkInfo> {
        WorkspaceService::with_config(self.config.clone()).fork(source, name)
    }

    /// Get the currently active vault name.
    pub fn current(&self) -> Result<Option<String>> {
        self.config.current_vault()
    }

    /// Open the current vault's storage backend.
    pub fn open_current(&self) -> Result<Arc<VaultBackend>> {
        let name = self
            .config
            .current_vault()?
            .ok_or(VfsError::NoActiveVault)?;
        self.open(&name)
    }

    /// Open a specific vault's storage backend.
    pub fn open(&self, name: &str) -> Result<Arc<VaultBackend>> {
        let backend = self
            .config
            .vault_backend(name)
            .ok_or_else(|| VfsError::VaultNotFound(name.to_string()))?;
        let path = self.config.vault_path_with_backend(name, backend);
        Ok(Arc::new(VaultBackend::open(&path, backend)?))
    }
}

impl Default for VaultManager {
    fn default() -> Self {
        Self::new().expect("failed to create vault manager")
    }
}
