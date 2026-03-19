//! Database compaction command.

use clap::Args;
use serde::Serialize;

use crate::commands::Output;
use crate::error::Result;
use crate::vault::VaultManager;

#[derive(Args)]
pub struct CompactArgs {}

#[derive(Serialize)]
struct CompactOutput {
    size_before: u64,
    size_after: u64,
    bytes_freed: u64,
    size_before_human: String,
    size_after_human: String,
    bytes_freed_human: String,
}

impl std::fmt::Display for CompactOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Compacted database: {} -> {} (freed {})",
            self.size_before_human, self.size_after_human, self.bytes_freed_human
        )
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

pub fn run(_args: CompactArgs, output: &Output, vault: Option<String>) -> Result<()> {
    let manager = VaultManager::new()?;
    let backend = match vault {
        Some(name) => manager.open(&name)?,
        None => manager.open_current()?,
    };

    // Get size before
    let size_before = backend.get_db_size()?;

    // Run VACUUM
    backend.vacuum()?;

    // Get size after
    let size_after = backend.get_db_size()?;

    let bytes_freed = size_before.saturating_sub(size_after);

    let result = CompactOutput {
        size_before,
        size_after,
        bytes_freed,
        size_before_human: format_size(size_before),
        size_after_human: format_size(size_after),
        bytes_freed_human: format_size(bytes_freed),
    };

    output.print(&result);

    Ok(())
}
