//! Vault statistics command.

use clap::Args;
use serde::Serialize;

use crate::commands::Output;
use crate::error::Result;
use crate::vault::VaultManager;

#[derive(Args)]
pub struct StatsArgs {}

#[derive(Serialize)]
struct StatsOutput {
    files: u64,
    directories: u64,
    total_versions: u64,
    content_blobs: u64,
    total_size_bytes: u64,
    total_size_human: String,
    orphaned_blobs: u64,
    orphaned_bytes: u64,
    orphaned_human: String,
    db_size_bytes: u64,
    db_size_human: String,
}

impl std::fmt::Display for StatsOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Vault Statistics")?;
        writeln!(f, "================")?;
        writeln!(f)?;
        writeln!(f, "Files:           {}", self.files)?;
        writeln!(f, "Directories:     {}", self.directories)?;
        writeln!(f, "Total versions:  {}", self.total_versions)?;
        writeln!(f)?;
        writeln!(f, "Content blobs:   {}", self.content_blobs)?;
        writeln!(f, "Content size:    {}", self.total_size_human)?;
        writeln!(f)?;
        writeln!(f, "Orphaned blobs:  {}", self.orphaned_blobs)?;
        writeln!(f, "Orphaned size:   {}", self.orphaned_human)?;
        writeln!(f)?;
        writeln!(f, "Database size:   {}", self.db_size_human)?;
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

pub fn run(_args: StatsArgs, output: &Output, vault: Option<String>) -> Result<()> {
    let manager = VaultManager::new()?;
    let backend = match vault {
        Some(name) => manager.open(&name)?,
        None => manager.open_current()?,
    };

    let stats = backend.get_vault_stats()?;
    let db_size = backend.get_db_size()?;

    let output_data = StatsOutput {
        files: stats.files,
        directories: stats.directories,
        total_versions: stats.total_versions,
        content_blobs: stats.content_blobs,
        total_size_bytes: stats.total_size_bytes,
        total_size_human: format_size(stats.total_size_bytes),
        orphaned_blobs: stats.orphaned_blobs,
        orphaned_bytes: stats.orphaned_bytes,
        orphaned_human: format_size(stats.orphaned_bytes),
        db_size_bytes: db_size,
        db_size_human: format_size(db_size),
    };

    output.print(&output_data);

    Ok(())
}
