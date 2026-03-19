//! untag command - remove tags from files.

use clap::Args;
use serde::Serialize;

use crate::commands::Output;
use crate::error::Result;
use crate::fs::FileSystem;
use crate::vault::VaultManager;

#[derive(Args)]
pub struct UntagArgs {
    /// Path to file
    pub path: String,

    /// Tag name to remove
    pub tag: String,
}

#[derive(Serialize)]
struct UntagOutput {
    path: String,
    tag: String,
    action: String,
}

pub fn run(args: UntagArgs, output: &Output, vault: Option<String>) -> Result<()> {
    let manager = VaultManager::new()?;
    let backend = match vault {
        Some(name) => manager.open(&name)?,
        None => manager.open_current()?,
    };

    let fs = FileSystem::new(backend.clone());

    let entry = fs.get_entry(&args.path)?;
    let tag = backend.get_tag_by_name(&args.tag)?.ok_or_else(|| {
        crate::error::VfsError::NotFound(std::path::PathBuf::from(format!("tag:{}", args.tag)))
    })?;

    backend.remove_tag_from_file(entry.id, tag.id)?;

    if output.is_json() {
        output.print_json(&UntagOutput {
            path: args.path,
            tag: args.tag,
            action: "removed".to_string(),
        });
    } else {
        println!("Removed tag '{}' from {}", args.tag, args.path);
    }

    Ok(())
}
