//! Quota management command.

use clap::{Args, Subcommand};
use serde::Serialize;

use crate::commands::Output;
use crate::error::Result;
use crate::vault::VaultManager;

#[derive(Args)]
pub struct QuotaArgs {
    #[command(subcommand)]
    pub command: Option<QuotaCommand>,
}

#[derive(Subcommand)]
pub enum QuotaCommand {
    /// Set a quota limit
    Set {
        /// Quota key (max_size_mb, max_files, max_file_size_mb)
        key: String,
        /// Quota value
        value: u64,
    },
    /// Clear a quota limit
    Clear {
        /// Quota key to clear
        key: String,
    },
}

#[derive(Serialize)]
struct QuotaOutput {
    current_size_bytes: u64,
    current_size_human: String,
    current_files: u64,
    max_size_mb: Option<u64>,
    max_files: Option<u64>,
    max_file_size_mb: Option<u64>,
    size_usage_percent: Option<f64>,
    files_usage_percent: Option<f64>,
}

impl std::fmt::Display for QuotaOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Quota Status")?;
        writeln!(f, "============")?;
        writeln!(f)?;
        writeln!(f, "Current usage:")?;
        writeln!(f, "  Files: {}", self.current_files)?;
        writeln!(f, "  Size:  {}", self.current_size_human)?;
        writeln!(f)?;
        writeln!(f, "Limits:")?;

        if let Some(max) = self.max_size_mb {
            let usage = self.size_usage_percent.unwrap_or(0.0);
            writeln!(f, "  Max size:      {} MB ({:.1}% used)", max, usage)?;
        } else {
            writeln!(f, "  Max size:      (unlimited)")?;
        }

        if let Some(max) = self.max_files {
            let usage = self.files_usage_percent.unwrap_or(0.0);
            writeln!(f, "  Max files:     {} ({:.1}% used)", max, usage)?;
        } else {
            writeln!(f, "  Max files:     (unlimited)")?;
        }

        if let Some(max) = self.max_file_size_mb {
            writeln!(f, "  Max file size: {} MB", max)?;
        } else {
            writeln!(f, "  Max file size: (unlimited)")?;
        }

        Ok(())
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

fn validate_quota_key(key: &str) -> Result<()> {
    match key {
        "max_size_mb" | "max_files" | "max_file_size_mb" => Ok(()),
        _ => Err(crate::error::VfsError::InvalidInput(format!(
            "invalid quota key: {}. Valid keys: max_size_mb, max_files, max_file_size_mb",
            key
        ))),
    }
}

pub fn run(args: QuotaArgs, output: &Output, vault: Option<String>) -> Result<()> {
    let manager = VaultManager::new()?;
    let backend = match vault {
        Some(name) => manager.open(&name)?,
        None => manager.open_current()?,
    };

    match args.command {
        Some(QuotaCommand::Set { key, value }) => {
            validate_quota_key(&key)?;
            backend.set_quota(&key, value)?;

            if output.is_json() {
                output.print_json(&serde_json::json!({
                    "action": "set",
                    "key": key,
                    "value": value
                }));
            } else {
                println!("Set {} = {}", key, value);
            }
        }
        Some(QuotaCommand::Clear { key }) => {
            validate_quota_key(&key)?;
            backend.clear_quota(&key)?;

            if output.is_json() {
                output.print_json(&serde_json::json!({
                    "action": "clear",
                    "key": key
                }));
            } else {
                println!("Cleared {}", key);
            }
        }
        None => {
            // Show quota status
            let stats = backend.get_vault_stats()?;
            let quotas = backend.get_all_quotas()?;

            let size_usage = quotas.max_size_mb.map(|max| {
                let max_bytes = max * 1024 * 1024;
                (stats.total_size_bytes as f64 / max_bytes as f64) * 100.0
            });

            let files_usage = quotas.max_files.map(|max| {
                (stats.files as f64 / max as f64) * 100.0
            });

            let result = QuotaOutput {
                current_size_bytes: stats.total_size_bytes,
                current_size_human: format_size(stats.total_size_bytes),
                current_files: stats.files,
                max_size_mb: quotas.max_size_mb,
                max_files: quotas.max_files,
                max_file_size_mb: quotas.max_file_size_mb,
                size_usage_percent: size_usage,
                files_usage_percent: files_usage,
            };

            output.print(&result);
        }
    }

    Ok(())
}
