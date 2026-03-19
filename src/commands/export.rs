//! export command - export files to real filesystem.

use std::fs;
use std::path::{Path, PathBuf};

use clap::Args;
use serde::Serialize;

use crate::commands::Output;
use crate::error::Result;
use crate::fs::FileSystem;
use crate::vault::VaultManager;

#[derive(Args)]
pub struct ExportArgs {
    /// Path in virtual filesystem
    pub vfs_path: String,

    /// Destination path on real filesystem
    pub real_path: String,

    /// Recursively export directories
    #[arg(short, long)]
    pub recursive: bool,

    /// Maximum depth for recursive export
    #[arg(long)]
    pub max_depth: Option<usize>,

    /// Overwrite existing files
    #[arg(long)]
    pub overwrite: bool,
}

#[derive(Serialize)]
struct ExportOutput {
    vfs_path: String,
    real_path: String,
    files_exported: usize,
    dirs_created: usize,
    total_bytes: u64,
}

struct ExportStats {
    files_exported: usize,
    dirs_created: usize,
    total_bytes: u64,
}

pub fn run(args: ExportArgs, output: &Output, vault: Option<String>) -> Result<()> {
    let manager = VaultManager::new()?;
    let backend = match vault {
        Some(name) => manager.open(&name)?,
        None => manager.open_current()?,
    };

    let vfs = FileSystem::new(backend);
    let real_path = PathBuf::from(&args.real_path);

    // Get vfs entry
    let entry = vfs.get_entry(&args.vfs_path)?;

    let mut stats = ExportStats {
        files_exported: 0,
        dirs_created: 0,
        total_bytes: 0,
    };

    if entry.is_file() {
        // Export single file
        export_file(&vfs, &args.vfs_path, &real_path, args.overwrite, &mut stats)?;
    } else if entry.is_dir() {
        if !args.recursive {
            return Err(crate::error::VfsError::Internal(
                "use -r to export directories recursively".to_string(),
            ));
        }
        // Export directory recursively
        export_recursive(&vfs, &args.vfs_path, &real_path, 0, args.max_depth, args.overwrite, &mut stats)?;
    }

    if output.is_json() {
        output.print_json(&ExportOutput {
            vfs_path: args.vfs_path,
            real_path: args.real_path,
            files_exported: stats.files_exported,
            dirs_created: stats.dirs_created,
            total_bytes: stats.total_bytes,
        });
    } else {
        if stats.dirs_created > 0 {
            println!(
                "Exported {} file(s) and {} dir(s) ({} bytes)",
                stats.files_exported, stats.dirs_created, stats.total_bytes
            );
        } else {
            println!(
                "Exported {} file(s) ({} bytes)",
                stats.files_exported, stats.total_bytes
            );
        }
    }

    Ok(())
}

fn export_file(
    vfs: &FileSystem,
    vfs_path: &str,
    real_path: &Path,
    overwrite: bool,
    stats: &mut ExportStats,
) -> Result<()> {
    // Check if file already exists
    if real_path.exists() && !overwrite {
        return Err(crate::error::VfsError::AlreadyExists(real_path.to_path_buf()));
    }

    // Ensure parent directory exists
    if let Some(parent) = real_path.parent() {
        fs::create_dir_all(parent).map_err(crate::error::VfsError::Io)?;
    }

    // Read from vfs and write to real fs
    let content = vfs.read_file(vfs_path)?;
    stats.total_bytes += content.len() as u64;
    fs::write(real_path, &content).map_err(crate::error::VfsError::Io)?;
    stats.files_exported += 1;

    Ok(())
}

fn export_recursive(
    vfs: &FileSystem,
    vfs_path: &str,
    real_path: &Path,
    depth: usize,
    max_depth: Option<usize>,
    overwrite: bool,
    stats: &mut ExportStats,
) -> Result<()> {
    // Check depth limit
    if let Some(max) = max_depth {
        if depth > max {
            return Ok(());
        }
    }

    // Create real directory
    fs::create_dir_all(real_path).map_err(crate::error::VfsError::Io)?;
    stats.dirs_created += 1;

    // List vfs directory entries
    let entries = vfs.list_dir(vfs_path)?;

    for entry in entries {
        // Build paths for this entry
        let child_vfs_path = if vfs_path == "/" {
            format!("/{}", entry.name)
        } else {
            format!("{}/{}", vfs_path, entry.name)
        };
        let child_real_path = real_path.join(&entry.name);

        if entry.file_type.is_file() {
            export_file(vfs, &child_vfs_path, &child_real_path, overwrite, stats)?;
        } else if entry.file_type.is_dir() {
            export_recursive(vfs, &child_vfs_path, &child_real_path, depth + 1, max_depth, overwrite, stats)?;
        }
    }

    Ok(())
}
