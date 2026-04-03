//! Execution request and result types.

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::error::{Result, VfsError};

#[derive(Debug, Clone, Serialize)]
pub enum CommandSpec {
    Argv(Vec<String>),
    Shell(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum CheckpointMode {
    Auto,
    Never,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExecutionRequest {
    pub vault: Option<String>,
    pub cwd: String,
    pub readonly: bool,
    pub keep_mount: bool,
    pub mountpoint: Option<PathBuf>,
    pub checkpoint_mode: CheckpointMode,
    pub command: CommandSpec,
}

impl ExecutionRequest {
    pub fn validate(&self) -> Result<()> {
        if !self.cwd.starts_with('/') {
            return Err(VfsError::InvalidPath(format!(
                "proxy cwd must be absolute: {}",
                self.cwd
            )));
        }

        match &self.command {
            CommandSpec::Argv(argv) if argv.is_empty() => {
                Err(VfsError::InvalidInput(
                    "proxy requires a command; use --shell or -- <command> ...".to_string(),
                ))
            }
            CommandSpec::Shell(cmd) if cmd.trim().is_empty() => Err(VfsError::InvalidInput(
                "proxy shell command cannot be empty".to_string(),
            )),
            _ => Ok(()),
        }
    }

    pub fn command_display(&self) -> String {
        match &self.command {
            CommandSpec::Argv(argv) => argv.join(" "),
            CommandSpec::Shell(cmd) => cmd.clone(),
        }
    }

    pub fn resolve_cwd(&self, mountpoint: &Path) -> Result<PathBuf> {
        if self.cwd == "/" {
            return Ok(mountpoint.to_path_buf());
        }

        Ok(mountpoint.join(self.cwd.trim_start_matches('/')))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum PolicyAction {
    Allow,
    AllowWithCheckpoint,
    Deny,
    RequireApproval,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub enum CommandCategory {
    ReadOnly,
    Mutating,
    Destructive,
    Networked,
    HostEscapeRisk,
    Interactive,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExecutionDecision {
    pub action: PolicyAction,
    pub categories: Vec<CommandCategory>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExecutionResult {
    pub vault: String,
    pub mountpoint: String,
    pub cwd: String,
    pub command: String,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub readonly: bool,
    pub kept_mounted: bool,
    pub duration_ms: u128,
    pub checkpoint: Option<String>,
    pub changed_files: Vec<String>,
    pub decision: ExecutionDecision,
}
