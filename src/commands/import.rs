//! import command - import files from real filesystem.

use std::fs;
use std::path::Path;
use std::sync::Arc;

use clap::Args;
use serde::Serialize;

use crate::commands::Output;
use crate::error::{Result, VfsError};
use crate::fs::FileSystem;
use crate::storage::SqliteBackend;
use crate::vault::VaultManager;

#[derive(Args)]
pub struct ImportArgs {
    /// Path on real filesystem
    pub real_path: String,

    /// Destination path in virtual filesystem
    pub vfs_path: String,

    /// Recursively import directories
    #[arg(short, long)]
    pub recursive: bool,

    /// Maximum depth for recursive import
    #[arg(long)]
    pub max_depth: Option<usize>,
}

#[derive(Serialize)]
struct ImportOutput {
    vfs_path: String,
    files_imported: usize,
    dirs_created: usize,
    total_bytes: u64,
}

struct ImportStats {
    files_imported: usize,
    dirs_created: usize,
    total_bytes: u64,
}

pub fn run(args: ImportArgs, output: &Output, vault: Option<String>) -> Result<()> {
    let manager = VaultManager::new()?;
    let backend = match vault {
        Some(name) => manager.open(&name)?,
        None => manager.open_current()?,
    };

    let vfs = FileSystem::new(backend.clone());
    let real_path = Path::new(&args.real_path);

    // Check if real path exists
    let metadata = fs::metadata(real_path).map_err(|e| {
        crate::error::VfsError::Io(e)
    })?;

    // Pre-calculate import size for quota checking
    let (total_size, file_count) = if metadata.is_file() {
        (metadata.len(), 1usize)
    } else if metadata.is_dir() {
        if !args.recursive {
            return Err(crate::error::VfsError::Internal(
                "use -r to import directories recursively".to_string(),
            ));
        }
        calculate_import_size(real_path, 0, args.max_depth)?
    } else {
        return Err(crate::error::VfsError::Internal(
            "not a file or directory".to_string(),
        ));
    };

    // Check quotas before importing
    // Check max file size for single file imports
    if metadata.is_file() {
        let max_file_size = backend.get_quota("max_file_size_mb")?;
        if let Some(max_mb) = max_file_size {
            let max_bytes = max_mb * 1024 * 1024;
            if total_size > max_bytes {
                return Err(VfsError::QuotaExceeded(format!(
                    "file size {} bytes exceeds max_file_size_mb ({}MB)",
                    total_size, max_mb
                )));
            }
        }
    }

    // Check total size and file count quotas
    let quota_check = backend.check_quota(total_size, file_count as u64)?;
    if !quota_check.allowed {
        return Err(VfsError::QuotaExceeded(
            quota_check.reason.unwrap_or_else(|| "quota exceeded".to_string()),
        ));
    }

    let mut stats = ImportStats {
        files_imported: 0,
        dirs_created: 0,
        total_bytes: 0,
    };

    if metadata.is_file() {
        // Import single file
        import_file(&vfs, &backend, real_path, &args.vfs_path, &mut stats)?;
    } else {
        // Import directory recursively (already checked -r flag above)
        import_recursive(&vfs, &backend, real_path, &args.vfs_path, 0, args.max_depth, &mut stats)?;
    }

    // Log the import operation
    let details = serde_json::json!({
        "source": args.real_path,
        "files": stats.files_imported,
        "dirs": stats.dirs_created,
        "bytes": stats.total_bytes
    });
    let _ = backend.log_operation("import", Some(&args.vfs_path), Some(&details.to_string()));

    if output.is_json() {
        output.print_json(&ImportOutput {
            vfs_path: args.vfs_path,
            files_imported: stats.files_imported,
            dirs_created: stats.dirs_created,
            total_bytes: stats.total_bytes,
        });
    } else {
        if stats.dirs_created > 0 {
            println!(
                "Imported {} file(s) and {} dir(s) ({} bytes)",
                stats.files_imported, stats.dirs_created, stats.total_bytes
            );
        } else {
            println!(
                "Imported {} file(s) ({} bytes)",
                stats.files_imported, stats.total_bytes
            );
        }
    }

    Ok(())
}

/// Calculate total size and file count for import quota checking.
fn calculate_import_size(
    real_path: &Path,
    depth: usize,
    max_depth: Option<usize>,
) -> Result<(u64, usize)> {
    if let Some(max) = max_depth {
        if depth > max {
            return Ok((0, 0));
        }
    }

    let mut total_size = 0u64;
    let mut file_count = 0usize;

    let entries = fs::read_dir(real_path).map_err(VfsError::Io)?;
    for entry in entries {
        let entry = entry.map_err(VfsError::Io)?;
        let metadata = entry.metadata().map_err(VfsError::Io)?;

        if metadata.is_file() {
            total_size += metadata.len();
            file_count += 1;
        } else if metadata.is_dir() {
            let (sub_size, sub_count) = calculate_import_size(
                &entry.path(),
                depth + 1,
                max_depth,
            )?;
            total_size += sub_size;
            file_count += sub_count;
        }
    }

    Ok((total_size, file_count))
}

fn import_file(
    vfs: &FileSystem,
    backend: &Arc<SqliteBackend>,
    real_path: &Path,
    vfs_path: &str,
    stats: &mut ImportStats,
) -> Result<()> {
    let content = fs::read(real_path).map_err(VfsError::Io)?;
    let size = content.len() as u64;

    // Check per-file size quota
    let max_file_size = backend.get_quota("max_file_size_mb")?;
    if let Some(max_mb) = max_file_size {
        let max_bytes = max_mb * 1024 * 1024;
        if size > max_bytes {
            return Err(VfsError::QuotaExceeded(format!(
                "file '{}' ({} bytes) exceeds max_file_size_mb ({}MB)",
                real_path.display(), size, max_mb
            )));
        }
    }

    stats.total_bytes += size;
    vfs.write_file(vfs_path, &content)?;
    stats.files_imported += 1;
    Ok(())
}

fn import_recursive(
    vfs: &FileSystem,
    backend: &Arc<SqliteBackend>,
    real_path: &Path,
    vfs_path: &str,
    depth: usize,
    max_depth: Option<usize>,
    stats: &mut ImportStats,
) -> Result<()> {
    // Check depth limit
    if let Some(max) = max_depth {
        if depth > max {
            return Ok(());
        }
    }

    // Create directory in vfs
    vfs.create_dir_all(vfs_path)?;
    stats.dirs_created += 1;

    // Read real directory entries
    let entries = fs::read_dir(real_path).map_err(VfsError::Io)?;

    for entry in entries {
        let entry = entry.map_err(VfsError::Io)?;
        let entry_path = entry.path();
        let entry_name = entry.file_name();
        let entry_name_str = entry_name.to_string_lossy();

        // Build vfs path for this entry
        let child_vfs_path = if vfs_path == "/" {
            format!("/{}", entry_name_str)
        } else {
            format!("{}/{}", vfs_path, entry_name_str)
        };

        let metadata = entry.metadata().map_err(VfsError::Io)?;

        if metadata.is_file() {
            import_file(vfs, backend, &entry_path, &child_vfs_path, stats)?;
        } else if metadata.is_dir() {
            import_recursive(vfs, backend, &entry_path, &child_vfs_path, depth + 1, max_depth, stats)?;
        }
        // Skip symlinks and other special files
    }

    Ok(())
}
