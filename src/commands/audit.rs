//! Audit log command.

use chrono::{DateTime, TimeZone, Utc};
use clap::{Args, Subcommand};
use serde::Serialize;

use crate::commands::Output;
use crate::error::Result;
use crate::vault::VaultManager;

#[derive(Args)]
pub struct AuditArgs {
    #[command(subcommand)]
    pub command: Option<AuditCommand>,

    /// Number of entries to show (default: 50)
    #[arg(long, default_value = "50")]
    pub limit: usize,

    /// Show entries since this timestamp (ISO 8601 or Unix timestamp)
    #[arg(long)]
    pub since: Option<String>,
}

#[derive(Subcommand)]
pub enum AuditCommand {
    /// Clear audit log entries
    Clear {
        /// Clear entries before this timestamp
        #[arg(long)]
        before: Option<String>,
    },
}

#[derive(Serialize)]
struct AuditEntryOutput {
    id: i64,
    timestamp: String,
    operation: String,
    path: Option<String>,
    details: Option<serde_json::Value>,
}

#[derive(Serialize)]
struct AuditListOutput {
    entries: Vec<AuditEntryOutput>,
    total_count: u64,
}

impl std::fmt::Display for AuditListOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.entries.is_empty() {
            writeln!(f, "No audit log entries")?;
            return Ok(());
        }

        writeln!(f, "Audit Log ({} total entries)", self.total_count)?;
        writeln!(f, "{}", "=".repeat(60))?;

        for entry in &self.entries {
            let path_str = entry.path.as_deref().unwrap_or("-");
            let details_str = entry
                .details
                .as_ref()
                .map(|d| d.to_string())
                .unwrap_or_default();

            if details_str.is_empty() {
                writeln!(
                    f,
                    "{} | {:12} | {}",
                    entry.timestamp, entry.operation, path_str
                )?;
            } else {
                writeln!(
                    f,
                    "{} | {:12} | {} | {}",
                    entry.timestamp, entry.operation, path_str, details_str
                )?;
            }
        }

        Ok(())
    }
}

fn parse_timestamp(s: &str) -> Option<i64> {
    // Try Unix timestamp first
    if let Ok(ts) = s.parse::<i64>() {
        return Some(ts);
    }

    // Try ISO 8601
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Some(dt.timestamp());
    }

    // Try simple date format YYYY-MM-DD
    if let Ok(dt) = chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return dt.and_hms_opt(0, 0, 0).map(|dt| dt.and_utc().timestamp());
    }

    None
}

pub fn run(args: AuditArgs, output: &Output, vault: Option<String>) -> Result<()> {
    let manager = VaultManager::new()?;
    let backend = match vault {
        Some(name) => manager.open(&name)?,
        None => manager.open_current()?,
    };

    match args.command {
        Some(AuditCommand::Clear { before }) => {
            let before_ts = before.as_ref().and_then(|s| parse_timestamp(s));

            if before.is_some() && before_ts.is_none() {
                return Err(crate::error::VfsError::InvalidInput(
                    "invalid timestamp format".to_string(),
                ));
            }

            let deleted = backend.clear_audit_log(before_ts)?;

            if output.is_json() {
                output.print_json(&serde_json::json!({
                    "action": "clear",
                    "entries_deleted": deleted
                }));
            } else {
                println!("Cleared {} audit log entries", deleted);
            }
        }
        None => {
            let since_ts = args.since.as_ref().and_then(|s| parse_timestamp(s));

            if args.since.is_some() && since_ts.is_none() {
                return Err(crate::error::VfsError::InvalidInput(
                    "invalid timestamp format for --since".to_string(),
                ));
            }

            let entries = backend.get_audit_log(args.limit, since_ts)?;
            let total_count = backend.get_audit_count()?;

            let output_entries: Vec<AuditEntryOutput> = entries
                .into_iter()
                .map(|e| {
                    let ts = Utc
                        .timestamp_opt(e.timestamp, 0)
                        .single()
                        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                        .unwrap_or_else(|| e.timestamp.to_string());

                    let details = e.details.and_then(|d| serde_json::from_str(&d).ok());

                    AuditEntryOutput {
                        id: e.id,
                        timestamp: ts,
                        operation: e.operation,
                        path: e.path,
                        details,
                    }
                })
                .collect();

            let result = AuditListOutput {
                entries: output_entries,
                total_count,
            };

            output.print(&result);
        }
    }

    Ok(())
}
