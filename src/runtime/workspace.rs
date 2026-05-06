//! Workspace and fork lifecycle services.

use std::collections::HashMap;
use std::fs;
#[cfg(target_os = "linux")]
use std::fs::{File, OpenOptions};
#[cfg(target_os = "linux")]
use std::os::fd::AsRawFd;
#[cfg(target_os = "macos")]
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::error::{Result, VfsError};
use crate::storage::{BackendType, VaultBackend};
use crate::vault::{Config, ForkInfo};

#[derive(Clone)]
pub struct WorkspaceService {
    config: Config,
    /// Cache of open vault backends so multiple callers receive the same
    /// `Arc<VaultBackend>` for a given path. Weak references let backends
    /// be dropped when no `Arc` holders remain.
    backend_cache: Arc<Mutex<HashMap<PathBuf, std::sync::Weak<VaultBackend>>>>,
}

#[derive(Debug, Clone)]
pub struct WorkspaceDescriptor {
    pub name: String,
    pub backend: BackendType,
    pub path: PathBuf,
}

impl WorkspaceService {
    pub fn new() -> Result<Self> {
        Ok(Self {
            config: Config::new()?,
            backend_cache: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    pub fn with_config(config: Config) -> Self {
        Self {
            config,
            backend_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn current_name(&self) -> Result<Option<String>> {
        self.config.current_vault()
    }

    pub fn resolve_name(&self, requested: Option<&str>) -> Result<String> {
        match requested {
            Some(name) => {
                if self.config.vault_exists(name) {
                    Ok(name.to_string())
                } else {
                    Err(VfsError::VaultNotFound(name.to_string()))
                }
            }
            None => self.current_name()?.ok_or(VfsError::NoActiveVault),
        }
    }

    pub fn describe(&self, name: &str) -> Result<WorkspaceDescriptor> {
        let backend = self
            .config
            .vault_backend(name)
            .ok_or_else(|| VfsError::VaultNotFound(name.to_string()))?;

        Ok(WorkspaceDescriptor {
            name: name.to_string(),
            backend,
            path: self.config.vault_path_with_backend(name, backend),
        })
    }

    pub fn open(&self, name: &str) -> Result<Arc<VaultBackend>> {
        let descriptor = self.describe(name)?;
        let path = descriptor.path;

        let mut cache = self.backend_cache.lock().unwrap();

        // Prune dead entries and check for an existing live backend.
        let mut to_remove = Vec::new();
        let mut existing = None;
        for (k, weak) in cache.iter() {
            if let Some(arc) = weak.upgrade() {
                if k == &path {
                    existing = Some(arc);
                    break;
                }
            } else {
                to_remove.push(k.clone());
            }
        }
        for k in to_remove {
            cache.remove(&k);
        }

        if let Some(arc) = existing {
            return Ok(arc);
        }

        let backend = Arc::new(VaultBackend::open(&path, descriptor.backend)?);
        cache.insert(path, Arc::downgrade(&backend));
        Ok(backend)
    }

    pub fn fork(&self, source: &str, name: &str) -> Result<ForkInfo> {
        validate_workspace_name(source)?;
        validate_workspace_name(name)?;

        let descriptor = self.describe(source)?;

        if self.config.vault_exists(name) {
            return Err(VfsError::VaultExists(name.to_string()));
        }

        let source_backend = self.open(source)?;
        source_backend.sync()?;
        drop(source_backend);

        let target_path = self
            .config
            .vault_path_with_backend(name, descriptor.backend);
        let mut copy_on_write = clone_path(&descriptor.path, &target_path)?;

        if matches!(descriptor.backend, BackendType::Sqlite) {
            copy_on_write &= clone_optional_sidecar(
                &descriptor.path.with_extension("avfs-wal"),
                &target_path.with_extension("avfs-wal"),
            )?;
            copy_on_write &= clone_optional_sidecar(
                &descriptor.path.with_extension("avfs-shm"),
                &target_path.with_extension("avfs-shm"),
            )?;
        }

        Ok(ForkInfo {
            source: source.to_string(),
            name: name.to_string(),
            backend: descriptor.backend,
            path: target_path,
            copy_on_write,
        })
    }
}

pub(crate) fn validate_workspace_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(VfsError::InvalidPath(
            "vault name cannot be empty".to_string(),
        ));
    }

    if name.len() > 64 {
        return Err(VfsError::InvalidPath(format!(
            "vault name too long: {} chars (max 64)",
            name.len()
        )));
    }

    for c in name.chars() {
        if !c.is_alphanumeric() && c != '-' && c != '_' {
            return Err(VfsError::InvalidPath(format!(
                "invalid character in vault name: {:?}",
                c
            )));
        }
    }

    if name.starts_with('.') || name.starts_with('-') {
        return Err(VfsError::InvalidPath(
            "vault name cannot start with . or -".to_string(),
        ));
    }

    Ok(())
}

pub(crate) fn dir_size(path: &Path) -> u64 {
    let mut size = 0;
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                size += fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
            } else if path.is_dir() {
                size += dir_size(&path);
            }
        }
    }
    size
}

fn clone_optional_sidecar(source: &Path, target: &Path) -> Result<bool> {
    if !source.exists() {
        return Ok(true);
    }

    clone_path(source, target)
}

fn clone_path(source: &Path, target: &Path) -> Result<bool> {
    let metadata = fs::metadata(source)?;

    if metadata.is_dir() {
        fs::create_dir_all(target)?;
        let mut copy_on_write = true;

        for entry in fs::read_dir(source)? {
            let entry = entry?;
            let child_source = entry.path();
            let child_target = target.join(entry.file_name());
            copy_on_write &= clone_path(&child_source, &child_target)?;
        }

        Ok(copy_on_write)
    } else if metadata.is_file() {
        clone_file(source, target)
    } else {
        Err(VfsError::InvalidInput(format!(
            "unsupported vault storage entry: {}",
            source.display()
        )))
    }
}

fn clone_file(source: &Path, target: &Path) -> Result<bool> {
    if try_clone_file_copy_on_write(source, target)? {
        return Ok(true);
    }

    fs::copy(source, target)?;
    fs::set_permissions(target, fs::metadata(source)?.permissions())?;
    Ok(false)
}

fn try_clone_file_copy_on_write(source: &Path, target: &Path) -> Result<bool> {
    #[cfg(target_os = "macos")]
    {
        let source_cstr = std::ffi::CString::new(source.as_os_str().as_bytes())
            .map_err(|_| VfsError::InvalidPath(source.display().to_string()))?;
        let target_cstr = std::ffi::CString::new(target.as_os_str().as_bytes())
            .map_err(|_| VfsError::InvalidPath(target.display().to_string()))?;

        let rc = unsafe { libc::clonefile(source_cstr.as_ptr(), target_cstr.as_ptr(), 0) };
        if rc == 0 {
            return Ok(true);
        }

        let err = std::io::Error::last_os_error();
        if clone_fallback_allowed(err.raw_os_error()) {
            return Ok(false);
        }

        return Err(err.into());
    }

    #[cfg(target_os = "linux")]
    {
        let source_file = File::open(source)?;
        let target_file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(target)?;

        let rc = unsafe {
            libc::ioctl(
                target_file.as_raw_fd(),
                libc::FICLONE,
                source_file.as_raw_fd(),
            )
        };
        if rc == 0 {
            return Ok(true);
        }

        let err = std::io::Error::last_os_error();
        drop(target_file);
        let _ = fs::remove_file(target);

        if clone_fallback_allowed(err.raw_os_error()) {
            return Ok(false);
        }

        return Err(err.into());
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        let _ = (source, target);
        Ok(false)
    }
}

fn clone_fallback_allowed(errno: Option<i32>) -> bool {
    matches!(
        errno,
        Some(libc::ENOTSUP)
            | Some(libc::EOPNOTSUPP)
            | Some(libc::EXDEV)
            | Some(libc::ENOSYS)
            | Some(libc::EINVAL)
            | Some(libc::ENOTTY)
    )
}
