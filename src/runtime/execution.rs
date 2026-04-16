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
#[serde(rename_all = "snake_case")]
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

    pub fn command_mode(&self) -> CommandMode {
        match &self.command {
            CommandSpec::Argv(_) => CommandMode::Argv,
            CommandSpec::Shell(_) => CommandMode::Shell,
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
#[serde(rename_all = "snake_case")]
pub enum PolicyAction {
    Allow,
    AllowWithCheckpoint,
    Deny,
    RequireApproval,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandCategory {
    ReadOnly,
    Mutating,
    Destructive,
    Networked,
    HostEscapeRisk,
    Interactive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandMode {
    Argv,
    Shell,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExecutionRequestView {
    pub vault: Option<String>,
    pub cwd: String,
    pub readonly: bool,
    pub keep_mount: bool,
    pub mountpoint: Option<String>,
    pub checkpoint_mode: CheckpointMode,
    pub command: String,
    pub command_mode: CommandMode,
}

impl From<&ExecutionRequest> for ExecutionRequestView {
    fn from(request: &ExecutionRequest) -> Self {
        Self {
            vault: request.vault.clone(),
            cwd: request.cwd.clone(),
            readonly: request.readonly,
            keep_mount: request.keep_mount,
            mountpoint: request
                .mountpoint
                .as_ref()
                .map(|mountpoint| mountpoint.display().to_string()),
            checkpoint_mode: request.checkpoint_mode,
            command: request.command_display(),
            command_mode: request.command_mode(),
        }
    }
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

#[derive(Debug, Clone, Serialize)]
pub struct ExecutionEnvelope {
    pub schema_version: u32,
    pub kind: &'static str,
    pub success: bool,
    pub request: ExecutionRequestView,
    pub result: ExecutionResult,
}

impl ExecutionEnvelope {
    pub const SCHEMA_VERSION: u32 = 1;
    pub const KIND: &'static str = "proxy_exec_result";

    pub fn new(request: &ExecutionRequest, result: ExecutionResult) -> Self {
        let success = result.exit_code == 0;

        Self {
            schema_version: Self::SCHEMA_VERSION,
            kind: Self::KIND,
            success,
            request: ExecutionRequestView::from(request),
            result,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{
        CheckpointMode, CommandCategory, CommandSpec, ExecutionDecision, ExecutionEnvelope,
        ExecutionRequest, ExecutionResult, PolicyAction,
    };

    #[test]
    fn execution_envelope_is_versioned_and_snake_case() {
        let request = ExecutionRequest {
            vault: Some("task-1".to_string()),
            cwd: "/workspace".to_string(),
            readonly: false,
            keep_mount: false,
            mountpoint: Some(PathBuf::from("/tmp/avfs-task-1")),
            checkpoint_mode: CheckpointMode::Auto,
            command: CommandSpec::Argv(vec!["cargo".to_string(), "test".to_string()]),
        };

        let result = ExecutionResult {
            vault: "task-1".to_string(),
            mountpoint: "/tmp/avfs-task-1".to_string(),
            cwd: "/workspace".to_string(),
            command: "cargo test".to_string(),
            exit_code: 1,
            stdout: "stdout".to_string(),
            stderr: "stderr".to_string(),
            readonly: false,
            kept_mounted: false,
            duration_ms: 42,
            checkpoint: Some("checkpoint-20260404-120000".to_string()),
            changed_files: vec!["/workspace/Cargo.lock".to_string()],
            decision: ExecutionDecision {
                action: PolicyAction::AllowWithCheckpoint,
                categories: vec![CommandCategory::Mutating],
                reason: None,
            },
        };

        let value = serde_json::to_value(ExecutionEnvelope::new(&request, result)).unwrap();

        assert_eq!(value["schema_version"], 1);
        assert_eq!(value["kind"], "proxy_exec_result");
        assert_eq!(value["success"], false);
        assert_eq!(value["request"]["command_mode"], "argv");
        assert_eq!(value["request"]["checkpoint_mode"], "auto");
        assert_eq!(value["result"]["decision"]["action"], "allow_with_checkpoint");
        assert_eq!(value["result"]["decision"]["categories"][0], "mutating");
    }
}
