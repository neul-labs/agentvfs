//! Mount utility helpers shared by commands.

use std::path::Path;
use std::process::Command;

use crate::error::{Result, VfsError};

pub fn unmount_path(mountpoint: &Path, lazy: bool) -> Result<()> {
    #[cfg(target_os = "linux")]
    let mut cmd = {
        let mut cmd = Command::new("fusermount");
        cmd.arg("-u");
        if lazy {
            cmd.arg("-z");
        }
        cmd
    };

    #[cfg(target_os = "macos")]
    let mut cmd = {
        let mut cmd = Command::new("umount");
        if lazy {
            cmd.arg("-f");
        }
        cmd
    };

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = (mountpoint, lazy);
        return Err(VfsError::Internal(
            "unmount is unsupported on this platform".to_string(),
        ));
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        let result = cmd
            .arg(mountpoint)
            .output()
            .map_err(|e| VfsError::Internal(format!("failed to run unmount helper: {}", e)))?;

        if result.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&result.stderr);
            Err(VfsError::Internal(format!(
                "unmount failed: {}",
                stderr.trim()
            )))
        }
    }
}
