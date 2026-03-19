//! meta command - manage file metadata.

use clap::Args;
use serde::Serialize;
use std::collections::HashMap;

use crate::commands::Output;
use crate::error::Result;
use crate::fs::FileSystem;
use crate::vault::VaultManager;

#[derive(Args)]
pub struct MetaArgs {
    /// Path to file
    pub path: String,

    /// Metadata key
    pub key: Option<String>,

    /// Metadata value (if setting)
    pub value: Option<String>,

    /// Delete a metadata key
    #[arg(short, long)]
    pub delete: bool,

    /// Export metadata as JSON
    #[arg(long)]
    pub export: bool,

    /// Import metadata from JSON string
    #[arg(long)]
    pub import: Option<String>,
}

#[derive(Serialize)]
struct MetaOutput {
    path: String,
    key: String,
    value: String,
    action: String,
}

#[derive(Serialize)]
struct MetaListOutput {
    path: String,
    metadata: HashMap<String, String>,
}

pub fn run(args: MetaArgs, output: &Output, vault: Option<String>) -> Result<()> {
    let manager = VaultManager::new()?;
    let backend = match vault {
        Some(name) => manager.open(&name)?,
        None => manager.open_current()?,
    };

    let fs = FileSystem::new(backend.clone());
    let entry = fs.get_entry(&args.path)?;

    // Handle --export: export metadata as JSON
    if args.export {
        let metadata = backend.get_all_metadata(entry.id)?;
        let map: HashMap<String, String> = metadata
            .into_iter()
            .map(|m| (m.key, m.value))
            .collect();

        if output.is_json() {
            output.print_json(&MetaListOutput {
                path: args.path,
                metadata: map,
            });
        } else {
            let json = serde_json::to_string_pretty(&map).unwrap_or_default();
            println!("{}", json);
        }
        return Ok(());
    }

    // Handle --import: import metadata from JSON
    if let Some(ref json_str) = args.import {
        let map: HashMap<String, String> = serde_json::from_str(json_str).map_err(|e| {
            crate::error::VfsError::Internal(format!("invalid JSON: {}", e))
        })?;

        for (key, value) in &map {
            backend.set_metadata(entry.id, key, value)?;
        }

        if output.is_json() {
            output.print_json(&serde_json::json!({
                "path": args.path,
                "imported": map.len(),
                "action": "imported"
            }));
        } else {
            println!("Imported {} metadata key(s) to {}", map.len(), args.path);
        }
        return Ok(());
    }

    // Handle --delete: delete metadata key
    if args.delete {
        let key = args.key.as_ref().ok_or_else(|| {
            crate::error::VfsError::Internal("key required for --delete".to_string())
        })?;

        backend.delete_metadata(entry.id, key)?;

        if output.is_json() {
            output.print_json(&MetaOutput {
                path: args.path,
                key: key.clone(),
                value: String::new(),
                action: "deleted".to_string(),
            });
        } else {
            println!("Deleted metadata '{}' from {}", key, args.path);
        }
        return Ok(());
    }

    // Handle: meta <path> <key> <value> - set metadata
    if let (Some(ref key), Some(ref value)) = (&args.key, &args.value) {
        backend.set_metadata(entry.id, key, value)?;

        if output.is_json() {
            output.print_json(&MetaOutput {
                path: args.path,
                key: key.clone(),
                value: value.clone(),
                action: "set".to_string(),
            });
        } else {
            println!("Set {}={} on {}", key, value, args.path);
        }
        return Ok(());
    }

    // Handle: meta <path> <key> - get single metadata value
    if let Some(ref key) = args.key {
        let value = backend.get_metadata(entry.id, key)?;

        if output.is_json() {
            output.print_json(&serde_json::json!({
                "path": args.path,
                "key": key,
                "value": value
            }));
        } else {
            match value {
                Some(v) => println!("{}", v),
                None => println!("(not set)"),
            }
        }
        return Ok(());
    }

    // Default: meta <path> - list all metadata
    let metadata = backend.get_all_metadata(entry.id)?;
    let map: HashMap<String, String> = metadata
        .into_iter()
        .map(|m| (m.key, m.value))
        .collect();

    if output.is_json() {
        output.print_json(&MetaListOutput {
            path: args.path,
            metadata: map,
        });
    } else {
        if map.is_empty() {
            println!("No metadata on {}", args.path);
        } else {
            println!("Metadata on {}:", args.path);
            for (key, value) in &map {
                println!("  {} = {}", key, value);
            }
        }
    }

    Ok(())
}
