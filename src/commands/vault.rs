//! Vault management commands.

use clap::{Args, Subcommand};
use serde::Serialize;

use crate::commands::Output;
use crate::error::Result;
use crate::runtime::workspace::WorkspaceService;
use crate::storage::BackendType;
use crate::vault::VaultManager;

#[derive(Args)]
pub struct VaultArgs {
    #[command(subcommand)]
    pub command: VaultCommand,
}

#[derive(Subcommand)]
pub enum VaultCommand {
    /// Create a new vault
    Create {
        /// Name of the vault to create
        name: String,
        /// Storage backend to use (sqlite or sled)
        #[arg(short, long, default_value = "sqlite")]
        backend: String,
    },
    /// List all vaults
    List,
    /// Switch to a different vault
    Use {
        /// Name of the vault to use
        name: String,
    },
    /// Delete a vault
    Delete {
        /// Name of the vault to delete
        name: String,
        /// Skip confirmation
        #[arg(short = 'y', long)]
        yes: bool,
    },
    /// Show information about current or specified vault
    Info {
        /// Name of the vault (defaults to current)
        name: Option<String>,
    },
    /// Fork an existing vault into a new vault
    Fork {
        /// Source vault name
        source: String,
        /// New vault name
        name: String,
        /// Switch to the fork after creating it
        #[arg(long)]
        r#use: bool,
    },
}

#[derive(Serialize)]
struct VaultListItem {
    name: String,
    current: bool,
    size: u64,
    backend: String,
}

#[derive(Serialize)]
struct VaultInfoOutput {
    name: String,
    current: bool,
    size: u64,
    created_at: Option<String>,
    path: String,
    backend: String,
}

#[derive(Serialize)]
struct VaultForkOutput {
    source: String,
    name: String,
    current: bool,
    backend: String,
    path: String,
    copy_on_write: bool,
}

pub fn run(args: VaultArgs, output: &Output) -> Result<()> {
    let manager = VaultManager::new()?;

    match args.command {
        VaultCommand::Create { name, backend } => {
            let backend_type: BackendType = backend.parse().map_err(|e: String| {
                crate::error::VfsError::InvalidPath(e)
            })?;

            // Check if the feature is enabled for the requested backend
            #[cfg(not(feature = "sled-backend"))]
            if matches!(backend_type, BackendType::Sqlite) == false {
                return Err(crate::error::VfsError::Internal(
                    "sled backend requires the 'sled-backend' feature to be enabled".to_string()
                ));
            }

            manager.create_with_backend(&name, backend_type)?;
            if output.is_json() {
                output.print_json(&serde_json::json!({
                    "created": name,
                    "current": true,
                    "backend": backend_type.to_string(),
                }));
            } else {
                println!("Created vault: {} (backend: {})", name, backend_type);
            }
        }
        VaultCommand::List => {
            let vaults = manager.list()?;
            if output.is_json() {
                let items: Vec<VaultListItem> = vaults
                    .iter()
                    .map(|v| VaultListItem {
                        name: v.name.clone(),
                        current: v.is_current,
                        size: v.size,
                        backend: v.backend.to_string(),
                    })
                    .collect();
                output.print_json(&items);
            } else {
                if vaults.is_empty() {
                    println!("No vaults found. Create one with: avfs vault create <name>");
                } else {
                    for vault in vaults {
                        let marker = if vault.is_current { "* " } else { "  " };
                        println!("{}{} ({})", marker, vault.name, vault.backend);
                    }
                }
            }
        }
        VaultCommand::Use { name } => {
            manager.use_vault(&name)?;
            if output.is_json() {
                output.print_json(&serde_json::json!({
                    "current": name,
                }));
            } else {
                println!("Now using vault: {}", name);
            }
        }
        VaultCommand::Delete { name, yes } => {
            if !yes && !output.is_json() {
                eprintln!("Warning: This will permanently delete vault '{}' and all its contents.", name);
                eprintln!("Use -y flag to confirm deletion.");
                return Ok(());
            }
            manager.delete(&name)?;
            if output.is_json() {
                output.print_json(&serde_json::json!({
                    "deleted": name,
                }));
            } else {
                println!("Deleted vault: {}", name);
            }
        }
        VaultCommand::Info { name } => {
            let vault_name = match name {
                Some(n) => n,
                None => manager.current()?.ok_or(crate::error::VfsError::NoActiveVault)?,
            };
            let info = manager.info(&vault_name)?;
            let config = crate::vault::Config::new()?;

            if output.is_json() {
                output.print_json(&VaultInfoOutput {
                    name: info.name.clone(),
                    current: info.is_current,
                    size: info.size,
                    created_at: info.created_at.map(|dt| dt.to_rfc3339()),
                    path: config.vault_path(&info.name).display().to_string(),
                    backend: info.backend.to_string(),
                });
            } else {
                println!("Vault: {}", info.name);
                println!("Backend: {}", info.backend);
                println!("Current: {}", if info.is_current { "yes" } else { "no" });
                println!("Size: {} bytes", info.size);
                if let Some(created) = info.created_at {
                    println!("Created: {}", created.format("%Y-%m-%d %H:%M:%S"));
                }
                println!("Path: {}", config.vault_path(&info.name).display());
            }
        }
        VaultCommand::Fork { source, name, r#use } => {
            let info = WorkspaceService::new()?.fork(&source, &name)?;

            if r#use {
                manager.use_vault(&name)?;
            }

            if output.is_json() {
                output.print_json(&VaultForkOutput {
                    source,
                    name,
                    current: r#use,
                    backend: info.backend.to_string(),
                    path: info.path.display().to_string(),
                    copy_on_write: info.copy_on_write,
                });
            } else {
                let clone_mode = if info.copy_on_write {
                    "copy-on-write"
                } else {
                    "full copy"
                };
                println!(
                    "Forked vault: {} -> {} ({}, backend: {})",
                    info.source, info.name, clone_mode, info.backend
                );
                if r#use {
                    println!("Now using vault: {}", name);
                }
            }
        }
    }

    Ok(())
}
