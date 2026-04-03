//! Library-level mount session for proxy execution.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use fuser::{BackgroundSession, MountOption};

use crate::error::{Result, VfsError};
use crate::fs::FileSystem;
use crate::mount::VfsFilesystem;
use crate::storage::VaultBackend;

pub struct MountSession {
    mountpoint: PathBuf,
    session: Option<BackgroundSession>,
    owned_mountpoint: bool,
}

impl MountSession {
    pub fn spawn(
        workspace_name: &str,
        backend: Arc<VaultBackend>,
        mountpoint: PathBuf,
        readonly: bool,
        allow_other: bool,
        create_mountpoint: bool,
    ) -> Result<Self> {
        prepare_mountpoint(&mountpoint, create_mountpoint)?;

        let fs = FileSystem::new(backend);
        let vfs_fs = VfsFilesystem::new(fs, readonly);
        let options = mount_options(workspace_name, readonly, allow_other);

        let session = fuser::spawn_mount2(vfs_fs, &mountpoint, &options)
            .map_err(|e| VfsError::Internal(format!("mount failed: {}", e)))?;

        Ok(Self {
            mountpoint,
            session: Some(session),
            owned_mountpoint: create_mountpoint,
        })
    }

    pub fn mount_foreground(
        workspace_name: &str,
        backend: Arc<VaultBackend>,
        mountpoint: &Path,
        readonly: bool,
        allow_other: bool,
    ) -> Result<()> {
        prepare_mountpoint(mountpoint, false)?;

        let fs = FileSystem::new(backend);
        let vfs_fs = VfsFilesystem::new(fs, readonly);
        let options = mount_options(workspace_name, readonly, allow_other);

        fuser::mount2(vfs_fs, mountpoint, &options)
            .map_err(|e| VfsError::Internal(format!("mount failed: {}", e)))?;
        Ok(())
    }

    pub fn mountpoint(&self) -> &Path {
        &self.mountpoint
    }
}

impl Drop for MountSession {
    fn drop(&mut self) {
        if let Some(session) = self.session.take() {
            drop(session);
        }

        if self.owned_mountpoint {
            let _ = fs::remove_dir(&self.mountpoint);
        }
    }
}

fn mount_options(workspace_name: &str, readonly: bool, allow_other: bool) -> Vec<MountOption> {
    let mut options = vec![MountOption::FSName(format!("vfs:{}", workspace_name))];

    if readonly {
        options.push(MountOption::RO);
    }

    if allow_other {
        options.push(MountOption::AllowOther);
    }

    options
}

fn prepare_mountpoint(mountpoint: &Path, create: bool) -> Result<()> {
    if mountpoint.exists() {
        if !mountpoint.is_dir() {
            return Err(VfsError::NotADirectory(mountpoint.to_path_buf()));
        }
        return Ok(());
    }

    if !create {
        return Err(VfsError::NotFound(mountpoint.to_path_buf()));
    }

    fs::create_dir_all(mountpoint)?;
    Ok(())
}
