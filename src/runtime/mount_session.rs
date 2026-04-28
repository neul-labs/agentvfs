//! Library-level mount session for proxy execution.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use fuser::{BackgroundSession, MountOption};

use crate::error::{Result, VfsError};
use crate::fs::FileSystem;
use crate::mount::VfsFilesystem;
use crate::storage::VaultBackend;

/// Lifecycle state of a mount session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MountState {
    Unmounted,
    Mounting,
    Mounted,
    Unmounting,
}

impl MountState {
    pub fn is_mounted(&self) -> bool {
        matches!(self, MountState::Mounted)
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, MountState::Unmounted | MountState::Unmounting)
    }
}

pub struct MountSession {
    mountpoint: PathBuf,
    session: Option<BackgroundSession>,
    owned_mountpoint: bool,
    state: MountState,
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

        let mut state = MountState::Mounting;

        let session = fuser::spawn_mount2(vfs_fs, &mountpoint, &options)
            .map_err(|e| VfsError::Internal(format!("mount failed: {}", e)))?;

        state = MountState::Mounted;

        Ok(Self {
            mountpoint,
            session: Some(session),
            owned_mountpoint: create_mountpoint,
            state,
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

    pub fn state(&self) -> MountState {
        self.state
    }

    /// Explicitly unmount the session, returning an error if unmount fails.
    /// Safe to call multiple times; subsequent calls are no-ops.
    pub fn unmount(&mut self) -> Result<()> {
        if self.state.is_terminal() {
            return Ok(());
        }

        self.state = MountState::Unmounting;

        if let Some(session) = self.session.take() {
            drop(session);
        }

        if self.owned_mountpoint {
            if let Err(e) = fs::remove_dir(&self.mountpoint) {
                // Only report error if directory still exists
                if self.mountpoint.exists() {
                    return Err(VfsError::Io(e));
                }
            }
        }

        self.state = MountState::Unmounted;
        Ok(())
    }
}

impl Drop for MountSession {
    fn drop(&mut self) {
        if let Err(e) = self.unmount() {
            eprintln!("avfs: mount session cleanup failed: {}", e);
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
