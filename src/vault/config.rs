//! Global vfs configuration.

use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{Result, VfsError};

/// Global vfs configuration directory.
pub struct Config {
    /// Base directory (~/.vfs)
    base_dir: PathBuf,
}

impl Config {
    /// Create a new config, initializing directories if needed.
    pub fn new() -> Result<Self> {
        let base_dir = Self::default_base_dir()?;
        let config = Self { base_dir };
        config.ensure_dirs()?;
        Ok(config)
    }

    /// Create config with a custom base directory (for testing).
    pub fn with_base_dir(base_dir: PathBuf) -> Result<Self> {
        let config = Self { base_dir };
        config.ensure_dirs()?;
        Ok(config)
    }

    /// Get the default base directory (~/.vfs).
    fn default_base_dir() -> Result<PathBuf> {
        let home = dirs::home_dir().ok_or_else(|| {
            VfsError::Internal("could not determine home directory".to_string())
        })?;
        Ok(home.join(".vfs"))
    }

    /// Ensure all required directories exist.
    fn ensure_dirs(&self) -> Result<()> {
        fs::create_dir_all(self.vaults_dir())?;
        Ok(())
    }

    /// Get the base directory.
    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    /// Get the vaults directory.
    pub fn vaults_dir(&self) -> PathBuf {
        self.base_dir.join("vaults")
    }

    /// Get the path to a vault's database file.
    pub fn vault_path(&self, name: &str) -> PathBuf {
        self.vaults_dir().join(format!("{}.vfs", name))
    }

    /// Get the path to the current vault file.
    pub fn current_vault_file(&self) -> PathBuf {
        self.base_dir.join("current")
    }

    /// Get the currently active vault name.
    pub fn current_vault(&self) -> Result<Option<String>> {
        let path = self.current_vault_file();
        if !path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&path)?;
        let name = content.trim();
        if name.is_empty() {
            return Ok(None);
        }

        // Verify vault exists
        if !self.vault_path(name).exists() {
            // Current vault was deleted, clear the reference
            fs::remove_file(&path)?;
            return Ok(None);
        }

        Ok(Some(name.to_string()))
    }

    /// Set the currently active vault.
    pub fn set_current_vault(&self, name: &str) -> Result<()> {
        // Verify vault exists
        if !self.vault_path(name).exists() {
            return Err(VfsError::VaultNotFound(name.to_string()));
        }

        fs::write(self.current_vault_file(), name)?;
        Ok(())
    }

    /// Clear the current vault selection.
    pub fn clear_current_vault(&self) -> Result<()> {
        let path = self.current_vault_file();
        if path.exists() {
            fs::remove_file(&path)?;
        }
        Ok(())
    }

    /// List all vault names.
    pub fn list_vaults(&self) -> Result<Vec<String>> {
        let mut vaults = Vec::new();
        let vaults_dir = self.vaults_dir();

        if !vaults_dir.exists() {
            return Ok(vaults);
        }

        for entry in fs::read_dir(&vaults_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "vfs") {
                if let Some(stem) = path.file_stem() {
                    vaults.push(stem.to_string_lossy().to_string());
                }
            }
        }

        vaults.sort();
        Ok(vaults)
    }

    /// Check if a vault exists.
    pub fn vault_exists(&self, name: &str) -> bool {
        self.vault_path(name).exists()
    }

    /// Delete a vault's database file.
    pub fn delete_vault_file(&self, name: &str) -> Result<()> {
        let path = self.vault_path(name);
        if !path.exists() {
            return Err(VfsError::VaultNotFound(name.to_string()));
        }

        // Also delete WAL and SHM files if they exist
        let wal_path = path.with_extension("vfs-wal");
        let shm_path = path.with_extension("vfs-shm");

        fs::remove_file(&path)?;
        let _ = fs::remove_file(&wal_path);
        let _ = fs::remove_file(&shm_path);

        // If this was the current vault, clear the selection
        if let Ok(Some(current)) = self.current_vault() {
            if current == name {
                self.clear_current_vault()?;
            }
        }

        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new().expect("failed to initialize config")
    }
}
