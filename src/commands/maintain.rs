//! Combined maintenance command.

use clap::Args;
use serde::Serialize;

use crate::commands::Output;
use crate::error::Result;
use crate::vault::VaultManager;

#[derive(Args)]
pub struct MaintainArgs {
    /// Keep N versions during prune
    #[arg(long)]
    pub keep: Option<u64>,

    /// Prune versions older than N days
    #[arg(long)]
    pub older_than: Option<u64>,

    /// Skip garbage collection
    #[arg(long)]
    pub skip_gc: bool,

    /// Skip compaction
    #[arg(long)]
    pub skip_compact: bool,
}

#[derive(Serialize)]
struct MaintainOutput {
    // Initial stats
    initial_files: u64,
    initial_versions: u64,
    initial_blobs: u64,
    initial_size_bytes: u64,

    // Operations performed
    versions_pruned: u64,
    orphans_deleted: u64,
    bytes_freed_gc: u64,
    bytes_freed_compact: u64,

    // Final stats
    final_files: u64,
    final_versions: u64,
    final_blobs: u64,
    final_size_bytes: u64,
}

impl std::fmt::Display for MaintainOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Maintenance Complete")?;
        writeln!(f, "===================")?;
        writeln!(f)?;
        writeln!(f, "Initial state:")?;
        writeln!(f, "  Files: {}, Versions: {}, Blobs: {}",
            self.initial_files, self.initial_versions, self.initial_blobs)?;
        writeln!(f, "  Size: {}", format_size(self.initial_size_bytes))?;
        writeln!(f)?;
        writeln!(f, "Operations:")?;
        writeln!(f, "  Versions pruned: {}", self.versions_pruned)?;
        writeln!(f, "  Orphans deleted: {} ({})",
            self.orphans_deleted, format_size(self.bytes_freed_gc))?;
        writeln!(f, "  Compaction saved: {}", format_size(self.bytes_freed_compact))?;
        writeln!(f)?;
        writeln!(f, "Final state:")?;
        writeln!(f, "  Files: {}, Versions: {}, Blobs: {}",
            self.final_files, self.final_versions, self.final_blobs)?;
        writeln!(f, "  Size: {}", format_size(self.final_size_bytes))?;
        Ok(())
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

pub fn run(args: MaintainArgs, output: &Output, vault: Option<String>) -> Result<()> {
    let manager = VaultManager::new()?;
    let backend = match vault {
        Some(name) => manager.open(&name)?,
        None => manager.open_current()?,
    };

    // Get initial stats
    let initial_stats = backend.get_vault_stats()?;
    let initial_db_size = backend.get_db_size()?;

    let mut versions_pruned = 0u64;
    let mut orphans_deleted = 0u64;
    let mut bytes_freed_gc = 0u64;

    // Step 1: Prune versions if requested
    if args.keep.is_some() || args.older_than.is_some() {
        let older_than_ts = args.older_than.map(|days| {
            let now = chrono::Utc::now().timestamp();
            now - (days as i64 * 24 * 60 * 60)
        });

        let prune_stats = backend.prune_all_versions(args.keep, older_than_ts)?;
        versions_pruned = prune_stats.versions_deleted;

        if !output.is_json() {
            println!("Pruned {} versions from {} files",
                prune_stats.versions_deleted, prune_stats.files_processed);
        }
    }

    // Step 2: Recalculate ref_counts and run GC
    if !args.skip_gc {
        backend.recalculate_ref_counts()?;
        let gc_stats = backend.delete_orphaned_blobs()?;
        orphans_deleted = gc_stats.orphans_deleted;
        bytes_freed_gc = gc_stats.bytes_freed;

        if !output.is_json() && gc_stats.orphans_deleted > 0 {
            println!("Deleted {} orphaned blobs ({})",
                gc_stats.orphans_deleted, format_size(gc_stats.bytes_freed));
        }
    }

    // Step 3: Compact database
    let bytes_freed_compact = if !args.skip_compact {
        let size_before = backend.get_db_size()?;
        backend.vacuum()?;
        let size_after = backend.get_db_size()?;
        let freed = size_before.saturating_sub(size_after);

        if !output.is_json() && freed > 0 {
            println!("Compacted database, freed {}", format_size(freed));
        }

        freed
    } else {
        0
    };

    // Get final stats
    let final_stats = backend.get_vault_stats()?;
    let final_db_size = backend.get_db_size()?;

    let result = MaintainOutput {
        initial_files: initial_stats.files,
        initial_versions: initial_stats.total_versions,
        initial_blobs: initial_stats.content_blobs,
        initial_size_bytes: initial_db_size,

        versions_pruned,
        orphans_deleted,
        bytes_freed_gc,
        bytes_freed_compact,

        final_files: final_stats.files,
        final_versions: final_stats.total_versions,
        final_blobs: final_stats.content_blobs,
        final_size_bytes: final_db_size,
    };

    output.print(&result);

    Ok(())
}
