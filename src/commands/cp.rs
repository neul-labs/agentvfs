//! cp command - copy files and directories.

use clap::Args;
use serde::Serialize;

use crate::commands::Output;
use crate::error::{Result, VfsError};
use crate::fs::FileSystem;
use crate::vault::VaultManager;

#[derive(Args)]
pub struct CpArgs {
    /// Source path
    pub source: String,

    /// Destination path
    pub destination: String,

    /// Copy directories recursively
    #[arg(short, long)]
    pub recursive: bool,
}

#[derive(Serialize)]
struct CpOutput {
    source: String,
    destination: String,
    copied: bool,
}

pub fn run(args: CpArgs, output: &Output, vault: Option<String>) -> Result<()> {
    let manager = VaultManager::new()?;
    let backend = match vault {
        Some(name) => manager.open(&name)?,
        None => manager.open_current()?,
    };

    let fs = FileSystem::new(backend.clone());

    // Check if source is a directory and -r flag is needed
    let entry = fs.get_entry(&args.source)?;
    if entry.is_dir() && !args.recursive {
        return Err(VfsError::NotAFile(std::path::PathBuf::from(&args.source)));
    }

    // Calculate copy size for quota checking
    let (logical_size, file_count) = if entry.is_dir() {
        calculate_tree_size(&fs, &args.source)?
    } else {
        (entry.size, 1usize)
    };

    // Copies reuse existing content blobs, so they only consume file-count quota.
    let quota_check = backend.check_quota(0, file_count as u64)?;
    if !quota_check.allowed {
        return Err(VfsError::QuotaExceeded(
            quota_check
                .reason
                .unwrap_or_else(|| "quota exceeded".to_string()),
        ));
    }

    fs.copy(&args.source, &args.destination)?;

    // Log the operation
    let details = serde_json::json!({
        "from": args.source,
        "to": args.destination,
        "logical_size": logical_size,
        "storage_added": 0,
        "files": file_count
    });
    let _ = backend.log_operation("copy", Some(&args.destination), Some(&details.to_string()));

    if output.is_json() {
        output.print_json(&CpOutput {
            source: args.source,
            destination: args.destination,
            copied: true,
        });
    }

    Ok(())
}

/// Calculate total size and file count for a path (file or directory tree).
fn calculate_tree_size(fs: &FileSystem, path: &str) -> Result<(u64, usize)> {
    let entry = fs.get_entry(path)?;

    if !entry.is_dir() {
        return Ok((entry.size, 1));
    }

    let mut total_size = 0u64;
    let mut file_count = 0usize;

    let children = fs.list_dir(path)?;
    for child in children {
        let child_path = if path == "/" {
            format!("/{}", child.name)
        } else {
            format!("{}/{}", path, child.name)
        };

        if child.file_type.is_dir() {
            let (sub_size, sub_count) = calculate_tree_size(fs, &child_path)?;
            total_size += sub_size;
            file_count += sub_count;
        } else {
            total_size += child.size;
            file_count += 1;
        }
    }

    Ok((total_size, file_count))
}
