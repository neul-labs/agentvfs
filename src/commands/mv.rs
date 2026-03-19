//! mv command - move/rename files and directories.

use clap::Args;
use serde::Serialize;

use crate::commands::Output;
use crate::error::Result;
use crate::fs::FileSystem;
use crate::vault::VaultManager;

#[derive(Args)]
pub struct MvArgs {
    /// Source path
    pub source: String,

    /// Destination path
    pub destination: String,
}

#[derive(Serialize)]
struct MvOutput {
    source: String,
    destination: String,
    moved: bool,
}

pub fn run(args: MvArgs, output: &Output, vault: Option<String>) -> Result<()> {
    let manager = VaultManager::new()?;
    let backend = match vault {
        Some(name) => manager.open(&name)?,
        None => manager.open_current()?,
    };

    let fs = FileSystem::new(backend.clone());
    fs.move_entry(&args.source, &args.destination)?;

    // Log the operation
    let details = serde_json::json!({
        "from": args.source,
        "to": args.destination
    });
    let _ = backend.log_operation("move", Some(&args.destination), Some(&details.to_string()));

    if output.is_json() {
        output.print_json(&MvOutput {
            source: args.source,
            destination: args.destination,
            moved: true,
        });
    }

    Ok(())
}
