//! checkout command - restore a file to a specific version.

use clap::Args;
use serde::Serialize;

use crate::commands::Output;
use crate::error::Result;
use crate::fs::FileSystem;
use crate::vault::VaultManager;

#[derive(Args)]
pub struct CheckoutArgs {
    /// Path to the file
    pub path: String,

    /// Version number to restore
    #[arg(short, long)]
    pub version: u64,
}

#[derive(Serialize)]
struct CheckoutOutput {
    path: String,
    restored_version: u64,
    new_version: u64,
}

pub fn run(args: CheckoutArgs, output: &Output, vault: Option<String>) -> Result<()> {
    let manager = VaultManager::new()?;
    let backend = match vault {
        Some(name) => manager.open(&name)?,
        None => manager.open_current()?,
    };

    let fs = FileSystem::new(backend.clone());
    let entry = fs.get_entry(&args.path)?;

    if !entry.is_file() {
        return Err(crate::error::VfsError::NotAFile(std::path::PathBuf::from(
            &args.path,
        )));
    }

    // Get the content from the specified version
    let content = backend.get_version_content(entry.id, args.version)?;

    // Write it back (this creates a new version)
    fs.write_file(&args.path, &content)?;

    // Get the new version number
    let new_version = backend.get_latest_version_number(entry.id)?;

    if output.is_json() {
        output.print_json(&CheckoutOutput {
            path: args.path,
            restored_version: args.version,
            new_version,
        });
    } else {
        println!(
            "Restored {} to version {} (now version {})",
            args.path, args.version, new_version
        );
    }

    Ok(())
}
