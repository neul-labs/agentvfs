//! ls command - list directory contents.

use clap::Args;
use serde::Serialize;

use crate::commands::Output;
use crate::error::Result;
use crate::fs::{DirEntry, FileSystem, FileType};
use crate::vault::VaultManager;

#[derive(Args)]
pub struct LsArgs {
    /// Path to list (defaults to /)
    #[arg(default_value = "/")]
    pub path: String,

    /// Show detailed listing
    #[arg(short, long)]
    pub long: bool,

    /// Show all files (including hidden)
    #[arg(short, long)]
    pub all: bool,
}

#[derive(Serialize)]
struct LsEntry {
    name: String,
    #[serde(rename = "type")]
    file_type: String,
    size: u64,
    modified: String,
}

pub fn run(args: LsArgs, output: &Output, vault: Option<String>) -> Result<()> {
    let manager = VaultManager::new()?;
    let backend = match vault {
        Some(name) => manager.open(&name)?,
        None => manager.open_current()?,
    };

    let fs = FileSystem::new(backend);
    let entries = fs.list_dir(&args.path)?;

    if output.is_json() {
        let items: Vec<LsEntry> = entries
            .iter()
            .map(|e| LsEntry {
                name: e.name.clone(),
                file_type: match e.file_type {
                    FileType::Directory => "directory".to_string(),
                    FileType::File => "file".to_string(),
                },
                size: e.size,
                modified: e.modified_at.to_rfc3339(),
            })
            .collect();
        output.print_json(&items);
    } else if args.long {
        print_long_listing(&entries);
    } else {
        print_short_listing(&entries);
    }

    Ok(())
}

fn print_short_listing(entries: &[DirEntry]) {
    for entry in entries {
        if entry.file_type.is_dir() {
            println!("{}/", entry.name);
        } else {
            println!("{}", entry.name);
        }
    }
}

fn print_long_listing(entries: &[DirEntry]) {
    for entry in entries {
        let type_char = if entry.file_type.is_dir() { 'd' } else { '-' };
        let size = format_size(entry.size);
        let date = entry.modified_at.format("%Y-%m-%d %H:%M");

        let name = if entry.file_type.is_dir() {
            format!("{}/", entry.name)
        } else {
            entry.name.clone()
        };

        println!("{} {:>8} {} {}", type_char, size, date, name);
    }
}

fn format_size(size: u64) -> String {
    if size < 1024 {
        format!("{}", size)
    } else if size < 1024 * 1024 {
        format!("{}K", size / 1024)
    } else if size < 1024 * 1024 * 1024 {
        format!("{}M", size / (1024 * 1024))
    } else {
        format!("{}G", size / (1024 * 1024 * 1024))
    }
}
