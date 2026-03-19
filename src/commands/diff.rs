//! diff command - compare files or versions.

use clap::Args;
use serde::Serialize;

use crate::commands::Output;
use crate::error::Result;
use crate::fs::FileSystem;
use crate::vault::VaultManager;

#[derive(Args)]
pub struct DiffArgs {
    /// First file path
    pub path1: String,

    /// Second file path (optional, if comparing versions of same file)
    pub path2: Option<String>,

    /// First version number (for same-file comparison)
    #[arg(long = "v1")]
    pub version1: Option<u64>,

    /// Second version number (for same-file comparison)
    #[arg(long = "v2")]
    pub version2: Option<u64>,

    /// Show unified diff format
    #[arg(short, long)]
    pub unified: bool,
}

#[derive(Serialize)]
struct DiffOutput {
    path1: String,
    path2: String,
    version1: Option<u64>,
    version2: Option<u64>,
    identical: bool,
    diff: Vec<DiffLine>,
}

#[derive(Serialize)]
struct DiffLine {
    line_type: String, // "add", "remove", "context"
    line_num: usize,
    content: String,
}

pub fn run(args: DiffArgs, output: &Output, vault: Option<String>) -> Result<()> {
    let manager = VaultManager::new()?;
    let backend = match vault {
        Some(name) => manager.open(&name)?,
        None => manager.open_current()?,
    };

    let fs = FileSystem::new(backend.clone());

    // Determine what we're comparing
    let (content1, label1) = if let Some(v1) = args.version1 {
        // Comparing versions of the same file
        let entry = fs.get_entry(&args.path1)?;
        if !entry.is_file() {
            return Err(crate::error::VfsError::NotAFile(
                std::path::PathBuf::from(&args.path1),
            ));
        }
        let content = backend.get_version_content(entry.id, v1)?;
        (content, format!("{} (v{})", args.path1, v1))
    } else {
        let content = fs.read_file(&args.path1)?;
        (content, args.path1.clone())
    };

    let (content2, label2) = if let Some(v2) = args.version2 {
        let entry = fs.get_entry(&args.path1)?;
        let content = backend.get_version_content(entry.id, v2)?;
        (content, format!("{} (v{})", args.path1, v2))
    } else if let Some(ref path2) = args.path2 {
        let content = fs.read_file(path2)?;
        (content, path2.clone())
    } else {
        // Compare current with previous version
        let entry = fs.get_entry(&args.path1)?;
        let latest = backend.get_latest_version_number(entry.id)?;
        if latest < 2 {
            return Err(crate::error::VfsError::Internal(
                "no previous version to compare".to_string(),
            ));
        }
        let content = backend.get_version_content(entry.id, latest - 1)?;
        (content, format!("{} (v{})", args.path1, latest - 1))
    };

    // Convert to text for comparison
    let text1 = String::from_utf8_lossy(&content1);
    let text2 = String::from_utf8_lossy(&content2);

    let lines1: Vec<&str> = text1.lines().collect();
    let lines2: Vec<&str> = text2.lines().collect();

    let identical = content1 == content2;

    if output.is_json() {
        let diff_lines = compute_diff_lines(&lines1, &lines2);
        output.print_json(&DiffOutput {
            path1: label1,
            path2: label2,
            version1: args.version1,
            version2: args.version2,
            identical,
            diff: diff_lines,
        });
    } else {
        if identical {
            println!("Files are identical");
        } else {
            println!("--- {}", label1);
            println!("+++ {}", label2);
            print_simple_diff(&lines1, &lines2);
        }
    }

    Ok(())
}

fn compute_diff_lines(lines1: &[&str], lines2: &[&str]) -> Vec<DiffLine> {
    let mut result = Vec::new();

    // Simple line-by-line diff (not a real diff algorithm)
    let max_len = lines1.len().max(lines2.len());
    for i in 0..max_len {
        let l1 = lines1.get(i);
        let l2 = lines2.get(i);

        match (l1, l2) {
            (Some(a), Some(b)) if a == b => {
                result.push(DiffLine {
                    line_type: "context".to_string(),
                    line_num: i + 1,
                    content: a.to_string(),
                });
            }
            (Some(a), Some(b)) => {
                result.push(DiffLine {
                    line_type: "remove".to_string(),
                    line_num: i + 1,
                    content: a.to_string(),
                });
                result.push(DiffLine {
                    line_type: "add".to_string(),
                    line_num: i + 1,
                    content: b.to_string(),
                });
            }
            (Some(a), None) => {
                result.push(DiffLine {
                    line_type: "remove".to_string(),
                    line_num: i + 1,
                    content: a.to_string(),
                });
            }
            (None, Some(b)) => {
                result.push(DiffLine {
                    line_type: "add".to_string(),
                    line_num: i + 1,
                    content: b.to_string(),
                });
            }
            (None, None) => {}
        }
    }

    result
}

fn print_simple_diff(lines1: &[&str], lines2: &[&str]) {
    let max_len = lines1.len().max(lines2.len());
    for i in 0..max_len {
        let l1 = lines1.get(i);
        let l2 = lines2.get(i);

        match (l1, l2) {
            (Some(a), Some(b)) if a == b => {
                println!(" {}", a);
            }
            (Some(a), Some(b)) => {
                println!("-{}", a);
                println!("+{}", b);
            }
            (Some(a), None) => {
                println!("-{}", a);
            }
            (None, Some(b)) => {
                println!("+{}", b);
            }
            (None, None) => {}
        }
    }
}
