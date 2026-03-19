//! mount command - mount a vault as a FUSE filesystem.

use std::path::Path;

use clap::Args;
use fuser::MountOption;
use serde::Serialize;

use crate::commands::Output;
use crate::error::{Result, VfsError};
use crate::fs::FileSystem;
use crate::mount::VfsFilesystem;
use crate::vault::VaultManager;

#[derive(Args)]
pub struct MountArgs {
    /// Name of the vault to mount
    pub vault: String,

    /// Mount point (directory)
    pub mountpoint: String,

    /// Run in foreground (don't daemonize)
    #[arg(short, long)]
    pub foreground: bool,

    /// Mount read-only
    #[arg(short, long)]
    pub readonly: bool,

    /// Allow other users to access the mount
    #[arg(long)]
    pub allow_other: bool,
}

#[derive(Serialize)]
struct MountOutput {
    vault: String,
    mountpoint: String,
    readonly: bool,
}

pub fn run(args: MountArgs, output: &Output) -> Result<()> {
    // Verify mountpoint exists and is a directory
    let mountpoint = Path::new(&args.mountpoint);
    if !mountpoint.exists() {
        return Err(VfsError::NotFound(mountpoint.to_path_buf()));
    }
    if !mountpoint.is_dir() {
        return Err(VfsError::NotADirectory(mountpoint.to_path_buf()));
    }

    // Open the vault
    let manager = VaultManager::new()?;
    let backend = manager.open(&args.vault)?;
    let fs = FileSystem::new(backend);

    // Create FUSE filesystem
    let vfs_fs = VfsFilesystem::new(fs, args.readonly);

    // Build mount options
    let mut options = vec![
        MountOption::FSName(format!("vfs:{}", args.vault)),
    ];

    if args.readonly {
        options.push(MountOption::RO);
    }

    if args.allow_other {
        options.push(MountOption::AllowOther);
    }

    if output.is_json() {
        output.print_json(&MountOutput {
            vault: args.vault.clone(),
            mountpoint: args.mountpoint.clone(),
            readonly: args.readonly,
        });
    }

    if args.foreground {
        // Run in foreground
        if !output.is_json() {
            println!("Mounting {} at {} (foreground mode)", args.vault, args.mountpoint);
            println!("Press Ctrl+C to unmount");
        }

        fuser::mount2(vfs_fs, &args.mountpoint, &options)
            .map_err(|e| VfsError::Internal(format!("mount failed: {}", e)))?;
    } else {
        // Fork to background using a simple approach
        // Note: For production, use a proper daemonization library
        if !output.is_json() {
            println!("Mounted {} at {}", args.vault, args.mountpoint);
            println!("Use 'vfs unmount {}' or 'fusermount -u {}' to unmount",
                args.mountpoint, args.mountpoint);
        }

        // For now, just run in foreground
        // TODO: Implement proper daemonization
        fuser::mount2(vfs_fs, &args.mountpoint, &options)
            .map_err(|e| VfsError::Internal(format!("mount failed: {}", e)))?;
    }

    Ok(())
}
