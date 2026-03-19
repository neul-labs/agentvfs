//! tag command - manage file tags.

use clap::Args;
use serde::Serialize;

use crate::commands::Output;
use crate::error::Result;
use crate::fs::FileSystem;
use crate::vault::VaultManager;

#[derive(Args)]
pub struct TagArgs {
    /// Path to file (for add/list operations)
    pub path: Option<String>,

    /// Tag name to add
    pub tag: Option<String>,

    /// List tags on file
    #[arg(short, long)]
    pub list: bool,

    /// List all tags in vault
    #[arg(long)]
    pub list_all: bool,

    /// Create a new tag
    #[arg(long)]
    pub create: Option<String>,

    /// Delete a tag
    #[arg(long)]
    pub delete: Option<String>,

    /// Rename a tag (format: old:new)
    #[arg(long)]
    pub rename: Option<String>,
}

#[derive(Serialize)]
struct TagOutput {
    tag: String,
    action: String,
}

#[derive(Serialize)]
struct TagListOutput {
    path: Option<String>,
    tags: Vec<String>,
}

#[derive(Serialize)]
struct AllTagsOutput {
    count: usize,
    tags: Vec<TagInfo>,
}

#[derive(Serialize)]
struct TagInfo {
    name: String,
    file_count: usize,
}

pub fn run(args: TagArgs, output: &Output, vault: Option<String>) -> Result<()> {
    let manager = VaultManager::new()?;
    let backend = match vault {
        Some(name) => manager.open(&name)?,
        None => manager.open_current()?,
    };

    let fs = FileSystem::new(backend.clone());

    // Handle --list-all: list all tags in vault
    if args.list_all {
        let tags = backend.list_tags()?;
        let tag_infos: Vec<TagInfo> = tags
            .iter()
            .map(|t| {
                let file_count = backend.get_files_with_tag(t.id).map(|f| f.len()).unwrap_or(0);
                TagInfo {
                    name: t.name.clone(),
                    file_count,
                }
            })
            .collect();

        if output.is_json() {
            output.print_json(&AllTagsOutput {
                count: tag_infos.len(),
                tags: tag_infos,
            });
        } else {
            if tag_infos.is_empty() {
                println!("No tags in vault");
            } else {
                println!("Tags in vault:");
                for t in tag_infos {
                    println!("  {} ({} files)", t.name, t.file_count);
                }
            }
        }
        return Ok(());
    }

    // Handle --create: create a new tag
    if let Some(ref tag_name) = args.create {
        if backend.get_tag_by_name(tag_name)?.is_some() {
            return Err(crate::error::VfsError::AlreadyExists(
                std::path::PathBuf::from(format!("tag:{}", tag_name)),
            ));
        }
        backend.create_tag(tag_name)?;

        if output.is_json() {
            output.print_json(&TagOutput {
                tag: tag_name.clone(),
                action: "created".to_string(),
            });
        } else {
            println!("Created tag: {}", tag_name);
        }
        return Ok(());
    }

    // Handle --delete: delete a tag
    if let Some(ref tag_name) = args.delete {
        let tag = backend.get_tag_by_name(tag_name)?.ok_or_else(|| {
            crate::error::VfsError::NotFound(std::path::PathBuf::from(format!("tag:{}", tag_name)))
        })?;
        backend.delete_tag(tag.id)?;

        if output.is_json() {
            output.print_json(&TagOutput {
                tag: tag_name.clone(),
                action: "deleted".to_string(),
            });
        } else {
            println!("Deleted tag: {}", tag_name);
        }
        return Ok(());
    }

    // Handle --rename: rename a tag
    if let Some(ref rename_spec) = args.rename {
        let parts: Vec<&str> = rename_spec.split(':').collect();
        if parts.len() != 2 {
            return Err(crate::error::VfsError::Internal(
                "rename format should be old:new".to_string(),
            ));
        }
        let old_name = parts[0];
        let new_name = parts[1];

        let tag = backend.get_tag_by_name(old_name)?.ok_or_else(|| {
            crate::error::VfsError::NotFound(std::path::PathBuf::from(format!("tag:{}", old_name)))
        })?;

        if backend.get_tag_by_name(new_name)?.is_some() {
            return Err(crate::error::VfsError::AlreadyExists(
                std::path::PathBuf::from(format!("tag:{}", new_name)),
            ));
        }

        backend.rename_tag(tag.id, new_name)?;

        if output.is_json() {
            output.print_json(&serde_json::json!({
                "old_name": old_name,
                "new_name": new_name,
                "action": "renamed"
            }));
        } else {
            println!("Renamed tag: {} -> {}", old_name, new_name);
        }
        return Ok(());
    }

    // Handle --list <path>: list tags on a file
    if args.list {
        let path = args.path.as_ref().ok_or_else(|| {
            crate::error::VfsError::Internal("path required for --list".to_string())
        })?;

        let entry = fs.get_entry(path)?;
        let tags = backend.get_file_tags(entry.id)?;
        let tag_names: Vec<String> = tags.iter().map(|t| t.name.clone()).collect();

        if output.is_json() {
            output.print_json(&TagListOutput {
                path: Some(path.clone()),
                tags: tag_names,
            });
        } else {
            if tag_names.is_empty() {
                println!("No tags on {}", path);
            } else {
                println!("Tags on {}:", path);
                for t in tag_names {
                    println!("  {}", t);
                }
            }
        }
        return Ok(());
    }

    // Default: add tag to file
    let path = args.path.as_ref().ok_or_else(|| {
        crate::error::VfsError::Internal("path required".to_string())
    })?;
    let tag_name = args.tag.as_ref().ok_or_else(|| {
        crate::error::VfsError::Internal("tag name required".to_string())
    })?;

    let entry = fs.get_entry(path)?;
    let tag_id = backend.get_or_create_tag(tag_name)?;
    backend.add_tag_to_file(entry.id, tag_id)?;

    if output.is_json() {
        output.print_json(&serde_json::json!({
            "path": path,
            "tag": tag_name,
            "action": "added"
        }));
    } else {
        println!("Tagged {} with '{}'", path, tag_name);
    }

    Ok(())
}
