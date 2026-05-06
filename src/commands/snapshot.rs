//! Snapshot management command.

use chrono::{TimeZone, Utc};
use clap::{Args, Subcommand};
use serde::Serialize;

use crate::commands::Output;
use crate::error::Result;
use crate::vault::VaultManager;

#[derive(Args)]
pub struct SnapshotArgs {
    #[command(subcommand)]
    pub command: SnapshotCommand,
}

#[derive(Subcommand)]
pub enum SnapshotCommand {
    /// Save current vault state as a snapshot
    Save {
        /// Snapshot name (auto-generated if not provided)
        name: Option<String>,

        /// Description of the snapshot
        #[arg(short, long)]
        description: Option<String>,
    },
    /// List all snapshots
    List,
    /// Restore vault to a snapshot state
    Restore {
        /// Snapshot name to restore
        name: String,
    },
    /// Delete a snapshot
    Delete {
        /// Snapshot name to delete
        name: String,
    },
    /// Show snapshot details
    Info {
        /// Snapshot name
        name: String,
    },
}

#[derive(Serialize)]
struct SnapshotOutput {
    name: String,
    created_at: String,
    file_count: u64,
    total_size: u64,
    total_size_human: String,
    description: Option<String>,
}

impl std::fmt::Display for SnapshotOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Snapshot: {}", self.name)?;
        writeln!(f, "Created:  {}", self.created_at)?;
        writeln!(f, "Files:    {}", self.file_count)?;
        writeln!(f, "Size:     {}", self.total_size_human)?;
        if let Some(desc) = &self.description {
            writeln!(f, "Description: {}", desc)?;
        }
        Ok(())
    }
}

#[derive(Serialize)]
struct SnapshotListOutput {
    snapshots: Vec<SnapshotOutput>,
}

impl std::fmt::Display for SnapshotListOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.snapshots.is_empty() {
            writeln!(f, "No snapshots")?;
            return Ok(());
        }

        writeln!(
            f,
            "{:<20} {:<20} {:>8} {:>12}",
            "NAME", "CREATED", "FILES", "SIZE"
        )?;
        writeln!(f, "{}", "-".repeat(64))?;

        for snap in &self.snapshots {
            writeln!(
                f,
                "{:<20} {:<20} {:>8} {:>12}",
                snap.name, snap.created_at, snap.file_count, snap.total_size_human
            )?;
        }

        Ok(())
    }
}

#[derive(Serialize)]
struct RestoreOutput {
    snapshot: String,
    files_restored: u64,
    dirs_restored: u64,
}

impl std::fmt::Display for RestoreOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Restored snapshot '{}': {} files, {} directories",
            self.snapshot, self.files_restored, self.dirs_restored
        )
    }
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} bytes", bytes)
    }
}

fn generate_snapshot_name() -> String {
    let now = Utc::now();
    now.format("snap-%Y%m%d-%H%M%S").to_string()
}

pub fn run(args: SnapshotArgs, output: &Output, vault: Option<String>) -> Result<()> {
    let manager = VaultManager::new()?;
    let backend = match vault {
        Some(name) => manager.open(&name)?,
        None => manager.open_current()?,
    };

    match args.command {
        SnapshotCommand::Save { name, description } => {
            let snap_name = name.unwrap_or_else(generate_snapshot_name);
            let info = backend.save_snapshot(&snap_name, description.as_deref())?;

            // Log the operation
            let details = serde_json::json!({
                "file_count": info.file_count,
                "total_size": info.total_size
            });
            let _ = backend.log_operation("snapshot_save", None, Some(&details.to_string()));

            let ts = Utc
                .timestamp_opt(info.created_at, 0)
                .single()
                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                .unwrap_or_else(|| info.created_at.to_string());

            let result = SnapshotOutput {
                name: info.name,
                created_at: ts,
                file_count: info.file_count,
                total_size: info.total_size,
                total_size_human: format_size(info.total_size),
                description: info.description,
            };

            if output.is_json() {
                output.print_json(&result);
            } else {
                println!("Saved snapshot: {}", result.name);
            }
        }
        SnapshotCommand::List => {
            let snapshots = backend.list_snapshots()?;

            let output_snapshots: Vec<SnapshotOutput> = snapshots
                .into_iter()
                .map(|s| {
                    let ts = Utc
                        .timestamp_opt(s.created_at, 0)
                        .single()
                        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                        .unwrap_or_else(|| s.created_at.to_string());

                    SnapshotOutput {
                        name: s.name,
                        created_at: ts,
                        file_count: s.file_count,
                        total_size: s.total_size,
                        total_size_human: format_size(s.total_size),
                        description: s.description,
                    }
                })
                .collect();

            let result = SnapshotListOutput {
                snapshots: output_snapshots,
            };

            output.print(&result);
        }
        SnapshotCommand::Restore { name } => {
            let stats = backend.restore_snapshot(&name)?;

            // Log the operation
            let details = serde_json::json!({
                "files_restored": stats.files_restored,
                "dirs_restored": stats.dirs_restored
            });
            let _ = backend.log_operation("snapshot_restore", None, Some(&details.to_string()));

            let result = RestoreOutput {
                snapshot: name,
                files_restored: stats.files_restored,
                dirs_restored: stats.dirs_restored,
            };

            output.print(&result);
        }
        SnapshotCommand::Delete { name } => {
            backend.delete_snapshot(&name)?;

            if output.is_json() {
                output.print_json(&serde_json::json!({
                    "action": "delete",
                    "name": name
                }));
            } else {
                println!("Deleted snapshot: {}", name);
            }
        }
        SnapshotCommand::Info { name } => {
            let info = backend.get_snapshot(&name)?;

            let ts = Utc
                .timestamp_opt(info.created_at, 0)
                .single()
                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                .unwrap_or_else(|| info.created_at.to_string());

            let result = SnapshotOutput {
                name: info.name,
                created_at: ts,
                file_count: info.file_count,
                total_size: info.total_size,
                total_size_human: format_size(info.total_size),
                description: info.description,
            };

            output.print(&result);
        }
    }

    Ok(())
}
