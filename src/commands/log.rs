//! log command - show version history.

use clap::Args;
use serde::Serialize;

use crate::commands::Output;
use crate::error::Result;
use crate::fs::FileSystem;
use crate::vault::VaultManager;

#[derive(Args)]
pub struct LogArgs {
    /// Path to the file
    pub path: String,

    /// Limit number of versions shown
    #[arg(short = 'n', long)]
    pub limit: Option<usize>,
}

#[derive(Serialize)]
struct LogEntry {
    version: u64,
    size: u64,
    created_at: String,
    hash: String,
}

pub fn run(args: LogArgs, output: &Output, vault: Option<String>) -> Result<()> {
    let manager = VaultManager::new()?;
    let backend = match vault {
        Some(name) => manager.open(&name)?,
        None => manager.open_current()?,
    };

    let fs = FileSystem::new(backend.clone());
    let entry = fs.get_entry(&args.path)?;

    if !entry.is_file() {
        return Err(crate::error::VfsError::NotAFile(
            std::path::PathBuf::from(&args.path),
        ));
    }

    let versions = backend.get_file_versions(entry.id)?;

    let versions: Vec<_> = match args.limit {
        Some(n) => versions.into_iter().take(n).collect(),
        None => versions,
    };

    if output.is_json() {
        let entries: Vec<LogEntry> = versions
            .iter()
            .map(|v| LogEntry {
                version: v.version_number,
                size: v.size,
                created_at: v.created_at.to_rfc3339(),
                hash: hex::encode(&v.content_hash[..8]),
            })
            .collect();
        output.print_json(&entries);
    } else {
        if versions.is_empty() {
            println!("No version history for {}", args.path);
        } else {
            println!("Version history for {}:", args.path);
            println!();
            for v in versions {
                let hash_short = hex::encode(&v.content_hash[..4]);
                let date = v.created_at.format("%Y-%m-%d %H:%M:%S");
                println!(
                    "  v{:<4} {} {:>8} bytes  {}",
                    v.version_number, date, v.size, hash_short
                );
            }
        }
    }

    Ok(())
}
