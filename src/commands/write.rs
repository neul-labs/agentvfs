//! write command - write content to a file.

use std::io::{self, Read};

use clap::Args;
use serde::Serialize;

use crate::commands::Output;
use crate::error::{Result, VfsError};
use crate::fs::FileSystem;
use crate::vault::VaultManager;

#[derive(Args)]
pub struct WriteArgs {
    /// Path to the file to write
    pub path: String,

    /// Content to write (if not provided, reads from stdin)
    pub content: Option<String>,

    /// Append to file instead of overwriting
    #[arg(short, long)]
    pub append: bool,
}

#[derive(Serialize)]
struct WriteOutput {
    path: String,
    size: usize,
    appended: bool,
}

pub fn run(args: WriteArgs, output: &Output, vault: Option<String>) -> Result<()> {
    let manager = VaultManager::new()?;
    let backend = match vault {
        Some(name) => manager.open(&name)?,
        None => manager.open_current()?,
    };

    let fs = FileSystem::new(backend.clone());

    // Get content from args or stdin
    let new_content = match args.content {
        Some(c) => c.into_bytes(),
        None => {
            let mut buffer = Vec::new();
            io::stdin().read_to_end(&mut buffer)?;
            buffer
        }
    };

    let final_content = if args.append {
        // Read existing content and append
        match fs.read_file(&args.path) {
            Ok(mut existing) => {
                existing.extend(new_content);
                existing
            }
            Err(_) => new_content,
        }
    } else {
        new_content
    };

    let size = final_content.len();

    // Check quotas
    let is_new_file = fs.get_entry(&args.path).is_err();
    let new_file_count = if is_new_file { 1 } else { 0 };
    let quota_check = backend.check_quota(size as u64, new_file_count)?;
    if !quota_check.allowed {
        return Err(VfsError::QuotaExceeded(
            quota_check
                .reason
                .unwrap_or_else(|| "quota exceeded".to_string()),
        ));
    }

    fs.write_file(&args.path, &final_content)?;

    // Log the operation
    let op = if is_new_file {
        "create_file"
    } else {
        "write_file"
    };
    let details = serde_json::json!({ "size": size });
    let _ = backend.log_operation(op, Some(&args.path), Some(&details.to_string()));

    if output.is_json() {
        output.print_json(&WriteOutput {
            path: args.path,
            size,
            appended: args.append,
        });
    } else {
        // Silent on success unless verbose
    }

    Ok(())
}
