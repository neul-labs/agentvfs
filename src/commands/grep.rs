//! grep command - regex content search.

use clap::Args;
use regex::Regex;
use serde::Serialize;

use crate::commands::Output;
use crate::error::Result;
use crate::fs::FileSystem;
use crate::vault::VaultManager;

#[derive(Args)]
pub struct GrepArgs {
    /// Regex pattern to search for
    pub pattern: String,

    /// Path to search in (defaults to /)
    #[arg(default_value = "/")]
    pub path: String,

    /// Case insensitive search
    #[arg(short, long)]
    pub ignore_case: bool,

    /// Show line numbers
    #[arg(short = 'n', long)]
    pub line_numbers: bool,

    /// Maximum number of results
    #[arg(short, long, default_value = "100")]
    pub limit: usize,
}

#[derive(Serialize)]
struct GrepOutput {
    pattern: String,
    count: usize,
    matches: Vec<GrepMatch>,
}

#[derive(Serialize)]
struct GrepMatch {
    path: String,
    line_number: Option<usize>,
    line: String,
}

pub fn run(args: GrepArgs, output: &Output, vault: Option<String>) -> Result<()> {
    let manager = VaultManager::new()?;
    let backend = match vault {
        Some(name) => manager.open(&name)?,
        None => manager.open_current()?,
    };

    let fs = FileSystem::new(backend);

    // Build regex
    let pattern = if args.ignore_case {
        format!("(?i){}", args.pattern)
    } else {
        args.pattern.clone()
    };

    let re = Regex::new(&pattern).map_err(|e| {
        crate::error::VfsError::Internal(format!("Invalid regex pattern: {}", e))
    })?;

    let mut matches = Vec::new();

    // Recursively search files
    search_directory(&fs, &args.path, &re, args.line_numbers, &mut matches, args.limit)?;

    if output.is_json() {
        output.print_json(&GrepOutput {
            pattern: args.pattern,
            count: matches.len(),
            matches,
        });
    } else {
        if matches.is_empty() {
            println!("No matches found for pattern: {}", args.pattern);
        } else {
            for m in &matches {
                if let Some(line_num) = m.line_number {
                    println!("\x1b[35m{}:\x1b[36m{}:\x1b[0m{}", m.path, line_num, m.line);
                } else {
                    println!("\x1b[35m{}:\x1b[0m{}", m.path, m.line);
                }
            }
            println!("\n{} match(es) found", matches.len());
        }
    }

    Ok(())
}

fn search_directory(
    fs: &FileSystem,
    path: &str,
    re: &Regex,
    show_line_numbers: bool,
    matches: &mut Vec<GrepMatch>,
    limit: usize,
) -> Result<()> {
    if matches.len() >= limit {
        return Ok(());
    }

    let entries = fs.list_dir(path)?;

    for entry in entries {
        if matches.len() >= limit {
            break;
        }

        let full_path = if path == "/" {
            format!("/{}", entry.name)
        } else {
            format!("{}/{}", path, entry.name)
        };

        if entry.file_type.is_dir() {
            search_directory(fs, &full_path, re, show_line_numbers, matches, limit)?;
        } else {
            // Try to read file content
            if let Ok(content) = fs.read_file(&full_path) {
                // Only search text files
                if let Ok(text) = String::from_utf8(content) {
                    for (line_num, line) in text.lines().enumerate() {
                        if matches.len() >= limit {
                            break;
                        }
                        if re.is_match(line) {
                            matches.push(GrepMatch {
                                path: full_path.clone(),
                                line_number: if show_line_numbers {
                                    Some(line_num + 1)
                                } else {
                                    None
                                },
                                line: line.to_string(),
                            });
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
