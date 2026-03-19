//! find command - find files by name or attributes.

use std::sync::Arc;

use clap::Args;
use glob::Pattern;
use serde::Serialize;

use crate::commands::Output;
use crate::error::Result;
use crate::fs::{FileSystem, FileType};
use crate::storage::SqliteBackend;
use crate::vault::VaultManager;

#[derive(Args)]
pub struct FindArgs {
    /// Starting path for search
    #[arg(default_value = "/")]
    pub path: String,

    /// Name pattern (glob syntax, e.g., "*.txt")
    #[arg(short, long)]
    pub name: Option<String>,

    /// File type: f (file), d (directory)
    #[arg(short = 't', long = "type")]
    pub file_type: Option<String>,

    /// Minimum size in bytes
    #[arg(long)]
    pub min_size: Option<u64>,

    /// Maximum size in bytes
    #[arg(long)]
    pub max_size: Option<u64>,

    /// Maximum depth to search
    #[arg(short, long)]
    pub depth: Option<usize>,

    /// Maximum number of results
    #[arg(short, long, default_value = "100")]
    pub limit: usize,

    /// Filter by tag
    #[arg(long)]
    pub tag: Option<String>,

    /// Filter by metadata (format: key=value)
    #[arg(long = "meta")]
    pub meta_filter: Option<String>,
}

#[derive(Serialize)]
struct FindOutput {
    count: usize,
    results: Vec<FindResult>,
}

#[derive(Serialize)]
struct FindResult {
    path: String,
    file_type: String,
    size: u64,
}

pub fn run(args: FindArgs, output: &Output, vault: Option<String>) -> Result<()> {
    let manager = VaultManager::new()?;
    let backend = match vault {
        Some(name) => manager.open(&name)?,
        None => manager.open_current()?,
    };

    let fs = FileSystem::new(backend.clone());

    // Parse name pattern
    let name_pattern = args.name.as_ref().map(|n| {
        Pattern::new(n).unwrap_or_else(|_| Pattern::new("*").unwrap())
    });

    // Parse file type filter
    let type_filter = args.file_type.as_ref().map(|t| match t.as_str() {
        "f" | "file" => FileType::File,
        "d" | "dir" | "directory" => FileType::Directory,
        _ => FileType::File,
    });

    // Parse tag filter - get tag ID if specified
    let tag_id = if let Some(ref tag_name) = args.tag {
        backend.get_tag_by_name(tag_name)?.map(|t| t.id)
    } else {
        None
    };

    // Parse metadata filter
    let meta_filter = args.meta_filter.as_ref().map(|m| {
        let parts: Vec<&str> = m.splitn(2, '=').collect();
        if parts.len() == 2 {
            (parts[0].to_string(), parts[1].to_string())
        } else {
            (m.clone(), String::new())
        }
    });

    let mut results = Vec::new();

    find_recursive(
        &fs,
        &backend,
        &args.path,
        0,
        &FindOptions {
            name_pattern,
            type_filter,
            min_size: args.min_size,
            max_size: args.max_size,
            max_depth: args.depth,
            limit: args.limit,
            tag_id,
            meta_filter,
        },
        &mut results,
    )?;

    if output.is_json() {
        output.print_json(&FindOutput {
            count: results.len(),
            results,
        });
    } else {
        if results.is_empty() {
            println!("No files found matching criteria");
        } else {
            for r in &results {
                println!("{}", r.path);
            }
        }
    }

    Ok(())
}

struct FindOptions {
    name_pattern: Option<Pattern>,
    type_filter: Option<FileType>,
    min_size: Option<u64>,
    max_size: Option<u64>,
    max_depth: Option<usize>,
    limit: usize,
    tag_id: Option<i64>,
    meta_filter: Option<(String, String)>,
}

fn find_recursive(
    fs: &FileSystem,
    backend: &Arc<SqliteBackend>,
    path: &str,
    depth: usize,
    options: &FindOptions,
    results: &mut Vec<FindResult>,
) -> Result<()> {
    if results.len() >= options.limit {
        return Ok(());
    }

    if let Some(max_depth) = options.max_depth {
        if depth > max_depth {
            return Ok(());
        }
    }

    let entries = fs.list_dir(path)?;

    for entry in entries {
        if results.len() >= options.limit {
            break;
        }

        let full_path = if path == "/" {
            format!("/{}", entry.name)
        } else {
            format!("{}/{}", path, entry.name)
        };

        let file_type = if entry.file_type.is_dir() {
            FileType::Directory
        } else {
            FileType::File
        };

        // Check filters
        let mut matches = true;

        // Name pattern
        if let Some(ref pattern) = options.name_pattern {
            if !pattern.matches(&entry.name) {
                matches = false;
            }
        }

        // File type
        if let Some(ref filter_type) = options.type_filter {
            if &file_type != filter_type {
                matches = false;
            }
        }

        // Size filters (only for files)
        if matches && file_type == FileType::File {
            if let Some(min) = options.min_size {
                if entry.size < min {
                    matches = false;
                }
            }
            if let Some(max) = options.max_size {
                if entry.size > max {
                    matches = false;
                }
            }
        }

        // Tag filter - need to get file_id first
        if matches && options.tag_id.is_some() {
            if let Ok(file_entry) = fs.get_entry(&full_path) {
                let file_tags = backend.get_file_tags(file_entry.id)?;
                let has_tag = file_tags.iter().any(|t| Some(t.id) == options.tag_id);
                if !has_tag {
                    matches = false;
                }
            } else {
                matches = false;
            }
        }

        // Metadata filter
        if matches {
            if let Some((ref key, ref value)) = options.meta_filter {
                if let Ok(file_entry) = fs.get_entry(&full_path) {
                    if let Ok(Some(stored_value)) = backend.get_metadata(file_entry.id, key) {
                        if !value.is_empty() && stored_value != *value {
                            matches = false;
                        }
                    } else {
                        matches = false;
                    }
                } else {
                    matches = false;
                }
            }
        }

        if matches {
            results.push(FindResult {
                path: full_path.clone(),
                file_type: match file_type {
                    FileType::File => "file".to_string(),
                    FileType::Directory => "directory".to_string(),
                },
                size: entry.size,
            });
        }

        // Recurse into directories
        if entry.file_type.is_dir() {
            find_recursive(fs, backend, &full_path, depth + 1, options, results)?;
        }
    }

    Ok(())
}
