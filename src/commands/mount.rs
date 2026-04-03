//! mount command - mount a vault as a FUSE filesystem.

use std::path::Path;

use clap::Args;
use fuser::MountOption;
use serde::Serialize;

use crate::commands::Output;
use crate::error::{Result, VfsError};
use crate::runtime::mount_session::MountSession;
use crate::runtime::workspace::WorkspaceService;
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

    let manager = VaultManager::new()?;
    let _ = manager.open(&args.vault)?;
    let backend = WorkspaceService::new()?.open(&args.vault)?;

    if output.is_json() {
        output.print_json(&MountOutput {
            vault: args.vault.clone(),
            mountpoint: args.mountpoint.clone(),
            readonly: args.readonly,
        });
    }

    if args.foreground {
        if !output.is_json() {
            println!("Mounting {} at {} (foreground mode)", args.vault, args.mountpoint);
            println!("Press Ctrl+C to unmount");
        }

        MountSession::mount_foreground(
            &args.vault,
            backend,
            mountpoint,
            args.readonly,
            args.allow_other,
        )?;
    } else {
        if !output.is_json() {
            println!("Mounted {} at {}", args.vault, args.mountpoint);
            println!("Use 'vfs unmount {}' or 'fusermount -u {}' to unmount",
                args.mountpoint, args.mountpoint);
        }

        let session = MountSession::spawn(
            &args.vault,
            backend,
            mountpoint.to_path_buf(),
            args.readonly,
            args.allow_other,
            false,
        )?;
        session
            .mountpoint()
            .to_path_buf();

        std::thread::park();
    }

    Ok(())
}
