//! cat command - read file contents.

use clap::Args;
use serde::Serialize;

use crate::commands::Output;
use crate::error::Result;
use crate::fs::FileSystem;
use crate::vault::VaultManager;

#[derive(Args)]
pub struct CatArgs {
    /// Path to the file to read
    pub path: String,

    /// Read a specific version
    #[arg(short, long)]
    pub version: Option<u64>,
}

#[derive(Serialize)]
struct CatOutput {
    path: String,
    content: String,
    size: usize,
    version: Option<u64>,
}

pub fn run(args: CatArgs, output: &Output, vault: Option<String>) -> Result<()> {
    let manager = VaultManager::new()?;
    let backend = match vault {
        Some(name) => manager.open(&name)?,
        None => manager.open_current()?,
    };

    let fs = FileSystem::new(backend.clone());

    let content = if let Some(version_num) = args.version {
        // Read specific version
        let entry = fs.get_entry(&args.path)?;
        if !entry.is_file() {
            return Err(crate::error::VfsError::NotAFile(
                std::path::PathBuf::from(&args.path),
            ));
        }
        backend.get_version_content(entry.id, version_num)?
    } else {
        // Read current version
        fs.read_file(&args.path)?
    };

    if output.is_json() {
        // Try to interpret as UTF-8
        let content_str = String::from_utf8_lossy(&content).to_string();
        output.print_json(&CatOutput {
            path: args.path,
            content: content_str,
            size: content.len(),
            version: args.version,
        });
    } else {
        // Print raw content
        match String::from_utf8(content.clone()) {
            Ok(text) => print!("{}", text),
            Err(_) => {
                // Binary file - print hex dump or error
                eprintln!("(binary file, {} bytes)", content.len());
            }
        }
    }

    Ok(())
}
