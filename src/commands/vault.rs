//! Vault management commands.

use clap::{Args, Subcommand};
use serde::Serialize;

use crate::commands::Output;
use crate::error::Result;
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
}

#[derive(Serialize)]
struct VaultListItem {
    name: String,
    current: bool,
    size: u64,
}

#[derive(Serialize)]
struct VaultInfoOutput {
    name: String,
    current: bool,
    size: u64,
    created_at: Option<String>,
    path: String,
}

pub fn run(args: VaultArgs, output: &Output) -> Result<()> {
    let manager = VaultManager::new()?;

    match args.command {
        VaultCommand::Create { name } => {
            manager.create(&name)?;
            if output.is_json() {
                output.print_json(&serde_json::json!({
                    "created": name,
                    "current": true,
                }));
            } else {
                println!("Created vault: {}", name);
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
                    })
                    .collect();
                output.print_json(&items);
            } else {
                if vaults.is_empty() {
                    println!("No vaults found. Create one with: vfs vault create <name>");
                } else {
                    for vault in vaults {
                        let marker = if vault.is_current { "* " } else { "  " };
                        println!("{}{}", marker, vault.name);
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
                });
            } else {
                println!("Vault: {}", info.name);
                println!("Current: {}", if info.is_current { "yes" } else { "no" });
                println!("Size: {} bytes", info.size);
                if let Some(created) = info.created_at {
                    println!("Created: {}", created.format("%Y-%m-%d %H:%M:%S"));
                }
                println!("Path: {}", config.vault_path(&info.name).display());
            }
        }
    }

    Ok(())
}
