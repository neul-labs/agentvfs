//! Garbage collection command.

use clap::Args;
use serde::Serialize;

use crate::commands::Output;
use crate::error::Result;
use crate::runtime::workspace::WorkspaceService;
use crate::storage::StorageMode;
use crate::vault::VaultManager;

#[derive(Args)]
pub struct GcArgs {
    /// Show what would be deleted without deleting
    #[arg(long)]
    pub dry_run: bool,
    /// (shared-blob stores) skip blobs created within this many seconds, to avoid racing a
    /// concurrent writer. Default 0 (assumes the store is quiescent).
    #[arg(long, default_value_t = 0)]
    pub grace_secs: i64,
}

#[derive(Serialize)]
struct GcOutput {
    orphans_found: u64,
    orphans_deleted: u64,
    bytes_freed: u64,
    bytes_freed_human: String,
    dry_run: bool,
}

impl std::fmt::Display for GcOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.dry_run {
            write!(
                f,
                "Found {} orphaned blobs ({}) that would be deleted (dry run)",
                self.orphans_found, self.bytes_freed_human
            )
        } else {
            write!(
                f,
                "Deleted {} orphaned blobs, freed {}",
                self.orphans_deleted, self.bytes_freed_human
            )
        }
    }
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} bytes", bytes)
    }
}

pub fn run(args: GcArgs, output: &Output, vault: Option<String>) -> Result<()> {
    let manager = VaultManager::new()?;
    let backend = match vault {
        Some(name) => manager.open(&name)?,
        None => manager.open_current()?,
    };

    // Shared-blob stores are reclaimed store-wide (mark-sweep across all vaults), since blobs
    // are owned by the store rather than any single vault.
    if backend.storage_mode() == StorageMode::SharedBlob {
        let (orphans, bytes) =
            WorkspaceService::new()?.gc_shared_store(args.grace_secs, args.dry_run)?;
        let result = GcOutput {
            orphans_found: orphans,
            orphans_deleted: if args.dry_run { 0 } else { orphans },
            bytes_freed: bytes,
            bytes_freed_human: format_size(bytes),
            dry_run: args.dry_run,
        };
        output.print(&result);
        return Ok(());
    }

    // Recalculate reference counts first
    backend.recalculate_ref_counts()?;

    if args.dry_run {
        // Just find orphans without deleting
        let orphans = backend.find_orphaned_blobs()?;
        let bytes: u64 = orphans.iter().map(|o| o.size).sum();

        let result = GcOutput {
            orphans_found: orphans.len() as u64,
            orphans_deleted: 0,
            bytes_freed: bytes,
            bytes_freed_human: format_size(bytes),
            dry_run: true,
        };
        output.print(&result);
    } else {
        // Delete orphans
        let stats = backend.delete_orphaned_blobs()?;

        let result = GcOutput {
            orphans_found: stats.orphans_found,
            orphans_deleted: stats.orphans_deleted,
            bytes_freed: stats.bytes_freed,
            bytes_freed_human: format_size(stats.bytes_freed),
            dry_run: false,
        };
        output.print(&result);
    }

    Ok(())
}
