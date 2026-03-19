//! mkdir command - create directories.

use clap::Args;
use serde::Serialize;

use crate::commands::Output;
use crate::error::Result;
use crate::fs::FileSystem;
use crate::vault::VaultManager;

#[derive(Args)]
pub struct MkdirArgs {
    /// Path of directory to create
    pub path: String,

    /// Create parent directories as needed
    #[arg(short, long)]
    pub parents: bool,
}

#[derive(Serialize)]
struct MkdirOutput {
    path: String,
    created: bool,
}

pub fn run(args: MkdirArgs, output: &Output, vault: Option<String>) -> Result<()> {
    let manager = VaultManager::new()?;
    let backend = match vault {
        Some(name) => manager.open(&name)?,
        None => manager.open_current()?,
    };

    let fs = FileSystem::new(backend.clone());

    if args.parents {
        fs.create_dir_all(&args.path)?;
    } else {
        fs.create_dir(&args.path)?;
    }

    // Log the operation
    let details = serde_json::json!({ "parents": args.parents });
    let _ = backend.log_operation("create_dir", Some(&args.path), Some(&details.to_string()));

    if output.is_json() {
        output.print_json(&MkdirOutput {
            path: args.path,
            created: true,
        });
    }

    Ok(())
}
