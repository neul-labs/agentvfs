//! Vault management operations.

use std::sync::Arc;

use chrono::{DateTime, TimeZone, Utc};

use crate::error::{Result, VfsError};
use crate::storage::SqliteBackend;
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

    /// Create a new vault.
    pub fn create(&self, name: &str) -> Result<()> {
        validate_vault_name(name)?;

        if self.config.vault_exists(name) {
            return Err(VfsError::VaultExists(name.to_string()));
        }

        // Create the database (schema is initialized in SqliteBackend::open)
        let path = self.config.vault_path(name);
        let _backend = SqliteBackend::open(&path)?;

        // If no current vault, set this as current
        if self.config.current_vault()?.is_none() {
            self.config.set_current_vault(name)?;
        }

        Ok(())
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
        if !self.config.vault_exists(name) {
            return Err(VfsError::VaultNotFound(name.to_string()));
        }

        let current = self.config.current_vault()?;
        let path = self.config.vault_path(name);

        let size = std::fs::metadata(&path)
            .map(|m| m.len())
            .unwrap_or(0);

        // Get creation timestamp from the database
        let created_at = if let Ok(backend) = SqliteBackend::open(&path) {
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
        };

        Ok(VaultInfo {
            name: name.to_string(),
            is_current: current.as_deref() == Some(name),
            size,
            created_at,
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

    /// Get the currently active vault name.
    pub fn current(&self) -> Result<Option<String>> {
        self.config.current_vault()
    }

    /// Open the current vault's storage backend.
    pub fn open_current(&self) -> Result<Arc<SqliteBackend>> {
        let name = self.config.current_vault()?.ok_or(VfsError::NoActiveVault)?;
        let path = self.config.vault_path(&name);
        Ok(Arc::new(SqliteBackend::open(&path)?))
    }

    /// Open a specific vault's storage backend.
    pub fn open(&self, name: &str) -> Result<Arc<SqliteBackend>> {
        if !self.config.vault_exists(name) {
            return Err(VfsError::VaultNotFound(name.to_string()));
        }

        let path = self.config.vault_path(name);
        Ok(Arc::new(SqliteBackend::open(&path)?))
    }
}

impl Default for VaultManager {
    fn default() -> Self {
        Self::new().expect("failed to create vault manager")
    }
}

/// Validate a vault name.
fn validate_vault_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(VfsError::InvalidPath("vault name cannot be empty".to_string()));
    }

    if name.len() > 64 {
        return Err(VfsError::InvalidPath(format!(
            "vault name too long: {} chars (max 64)",
            name.len()
        )));
    }

    // Only allow alphanumeric, hyphen, and underscore
    for c in name.chars() {
        if !c.is_alphanumeric() && c != '-' && c != '_' {
            return Err(VfsError::InvalidPath(format!(
                "invalid character in vault name: {:?}",
                c
            )));
        }
    }

    // Don't allow names starting with dot or hyphen
    if name.starts_with('.') || name.starts_with('-') {
        return Err(VfsError::InvalidPath(
            "vault name cannot start with . or -".to_string(),
        ));
    }

    Ok(())
}
