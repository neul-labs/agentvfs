//! revert command - revert a file to its previous version.

use clap::Args;
use serde::Serialize;

use crate::commands::Output;
use crate::error::Result;
use crate::fs::FileSystem;
use crate::vault::VaultManager;

#[derive(Args)]
pub struct RevertArgs {
    /// Path to the file
    pub path: String,
}

#[derive(Serialize)]
struct RevertOutput {
    path: String,
    reverted_from: u64,
    reverted_to: u64,
    new_version: u64,
}

pub fn run(args: RevertArgs, output: &Output, vault: Option<String>) -> Result<()> {
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

    // Get current version number
    let current_version = backend.get_latest_version_number(entry.id)?;

    if current_version < 2 {
        return Err(crate::error::VfsError::Internal(
            "no previous version to revert to".to_string(),
        ));
    }

    let previous_version = current_version - 1;

    // Get the content from the previous version
    let content = backend.get_version_content(entry.id, previous_version)?;

    // Write it back (this creates a new version)
    fs.write_file(&args.path, &content)?;

    // Get the new version number
    let new_version = backend.get_latest_version_number(entry.id)?;

    if output.is_json() {
        output.print_json(&RevertOutput {
            path: args.path,
            reverted_from: current_version,
            reverted_to: previous_version,
            new_version,
        });
    } else {
        println!(
            "Reverted {} from version {} to version {} (now version {})",
            args.path, current_version, previous_version, new_version
        );
    }

    Ok(())
}
