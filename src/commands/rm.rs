//! rm command - remove files and directories.

use clap::Args;
use serde::Serialize;

use crate::commands::Output;
use crate::error::Result;
use crate::fs::FileSystem;
use crate::vault::VaultManager;

#[derive(Args)]
pub struct RmArgs {
    /// Path to remove
    pub path: String,

    /// Remove directories and their contents recursively
    #[arg(short, long)]
    pub recursive: bool,

    /// Force removal (no error if file doesn't exist)
    #[arg(short, long)]
    pub force: bool,
}

#[derive(Serialize)]
struct RmOutput {
    path: String,
    removed: bool,
}

pub fn run(args: RmArgs, output: &Output, vault: Option<String>) -> Result<()> {
    let manager = VaultManager::new()?;
    let backend = match vault {
        Some(name) => manager.open(&name)?,
        None => manager.open_current()?,
    };

    let fs = FileSystem::new(backend.clone());

    // Get entry info before removal for audit logging
    let entry_info = fs.get_entry(&args.path).ok();

    match fs.remove(&args.path, args.recursive) {
        Ok(()) => {
            // Log the operation
            let op = if entry_info.as_ref().map(|e| e.is_dir()).unwrap_or(false) {
                "delete_dir"
            } else {
                "delete_file"
            };
            let details = if let Some(entry) = &entry_info {
                Some(serde_json::json!({
                    "size": entry.size,
                    "recursive": args.recursive
                }).to_string())
            } else {
                None
            };
            let _ = backend.log_operation(op, Some(&args.path), details.as_deref());

            if output.is_json() {
                output.print_json(&RmOutput {
                    path: args.path,
                    removed: true,
                });
            }
        }
        Err(crate::error::VfsError::NotFound(_)) if args.force => {
            if output.is_json() {
                output.print_json(&RmOutput {
                    path: args.path,
                    removed: false,
                });
            }
        }
        Err(e) => return Err(e),
    }

    Ok(())
}
