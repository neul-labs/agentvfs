//! Version pruning command.

use clap::Args;
use serde::Serialize;

use crate::commands::Output;
use crate::error::{Result, VfsError};
use crate::vault::VaultManager;

#[derive(Args)]
pub struct PruneArgs {
    /// File path to prune (required unless --all)
    pub path: Option<String>,

    /// Keep last N versions per file
    #[arg(long)]
    pub keep: Option<u64>,

    /// Delete versions older than N days
    #[arg(long)]
    pub older_than: Option<u64>,

    /// Prune all files in vault
    #[arg(long)]
    pub all: bool,

    /// Show what would be deleted without deleting
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Serialize)]
struct PruneOutput {
    files_processed: u64,
    versions_deleted: u64,
    dry_run: bool,
}

impl std::fmt::Display for PruneOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.dry_run {
            write!(
                f,
                "Would delete {} versions from {} files (dry run)",
                self.versions_deleted, self.files_processed
            )
        } else {
            write!(
                f,
                "Deleted {} versions from {} files",
                self.versions_deleted, self.files_processed
            )
        }
    }
}

pub fn run(args: PruneArgs, output: &Output, vault: Option<String>) -> Result<()> {
    // Validate arguments
    if args.keep.is_none() && args.older_than.is_none() {
        return Err(VfsError::InvalidInput(
            "must specify --keep or --older-than".to_string(),
        ));
    }

    if args.keep.is_some() && args.older_than.is_some() {
        return Err(VfsError::InvalidInput(
            "cannot specify both --keep and --older-than".to_string(),
        ));
    }

    if !args.all && args.path.is_none() {
        return Err(VfsError::InvalidInput(
            "must specify a path or use --all".to_string(),
        ));
    }

    let manager = VaultManager::new()?;
    let backend = match vault {
        Some(name) => manager.open(&name)?,
        None => manager.open_current()?,
    };

    // Calculate timestamp for --older-than
    let older_than_ts = args.older_than.map(|days| {
        let now = chrono::Utc::now().timestamp();
        now - (days as i64 * 24 * 60 * 60)
    });

    if args.all {
        // Prune all files
        if args.dry_run {
            // Count what would be deleted
            let file_ids = backend.get_all_file_ids()?;
            let mut total_to_delete = 0u64;
            let mut files_with_deletions = 0u64;

            for file_id in &file_ids {
                let count = if let Some(keep) = args.keep {
                    backend.count_versions_to_prune_keep(*file_id, keep)?
                } else if let Some(ts) = older_than_ts {
                    backend.count_versions_to_prune_older(*file_id, ts)?
                } else {
                    0
                };

                if count > 0 {
                    files_with_deletions += 1;
                    total_to_delete += count;
                }
            }

            let result = PruneOutput {
                files_processed: files_with_deletions,
                versions_deleted: total_to_delete,
                dry_run: true,
            };
            output.print(&result);
        } else {
            // Actually prune
            let stats = backend.prune_all_versions(args.keep, older_than_ts)?;

            let result = PruneOutput {
                files_processed: stats.files_processed,
                versions_deleted: stats.versions_deleted,
                dry_run: false,
            };
            output.print(&result);
        }
    } else {
        // Prune single file
        let path = args.path.as_ref().unwrap();
        let entry = backend.get_entry_by_path(path)?;

        if entry.is_dir() {
            return Err(VfsError::InvalidInput(
                "cannot prune a directory".to_string(),
            ));
        }

        if args.dry_run {
            let count = if let Some(keep) = args.keep {
                backend.count_versions_to_prune_keep(entry.id, keep)?
            } else if let Some(ts) = older_than_ts {
                backend.count_versions_to_prune_older(entry.id, ts)?
            } else {
                0
            };

            let result = PruneOutput {
                files_processed: if count > 0 { 1 } else { 0 },
                versions_deleted: count,
                dry_run: true,
            };
            output.print(&result);
        } else {
            let deleted = if let Some(keep) = args.keep {
                backend.prune_versions_keep(entry.id, keep)?
            } else if let Some(ts) = older_than_ts {
                backend.prune_versions_older_than(entry.id, ts)?
            } else {
                0
            };

            let result = PruneOutput {
                files_processed: if deleted > 0 { 1 } else { 0 },
                versions_deleted: deleted,
                dry_run: false,
            };
            output.print(&result);
        }
    }

    Ok(())
}
