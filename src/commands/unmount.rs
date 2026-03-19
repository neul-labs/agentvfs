//! unmount command - unmount a FUSE filesystem.

use std::process::Command;

use clap::Args;
use serde::Serialize;

use crate::commands::Output;
use crate::error::{Result, VfsError};

#[derive(Args)]
pub struct UnmountArgs {
    /// Mount point to unmount
    pub mountpoint: String,

    /// Lazy unmount (detach now, cleanup later)
    #[arg(short, long)]
    pub lazy: bool,
}

#[derive(Serialize)]
struct UnmountOutput {
    mountpoint: String,
    success: bool,
}

pub fn run(args: UnmountArgs, output: &Output) -> Result<()> {
    // Use fusermount to unmount
    let mut cmd = Command::new("fusermount");
    cmd.arg("-u");

    if args.lazy {
        cmd.arg("-z");
    }

    cmd.arg(&args.mountpoint);

    let result = cmd.output()
        .map_err(|e| VfsError::Internal(format!("failed to run fusermount: {}", e)))?;

    if result.status.success() {
        if output.is_json() {
            output.print_json(&UnmountOutput {
                mountpoint: args.mountpoint,
                success: true,
            });
        } else {
            println!("Unmounted {}", args.mountpoint);
        }
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&result.stderr);
        Err(VfsError::Internal(format!("unmount failed: {}", stderr.trim())))
    }
}
