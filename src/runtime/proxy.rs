//! Proxy runtime orchestration.

use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;

use crate::error::{Result, VfsError};
use crate::runtime::change_summary::ChangeSummaryService;
use crate::runtime::checkpoint::CheckpointService;
use crate::runtime::execution::{
    CheckpointMode, CommandSpec, ExecutionRequest, ExecutionResult, PolicyAction,
};
use crate::runtime::mount_session::MountSession;
use crate::runtime::policy::PolicyEngine;
use crate::runtime::workspace::WorkspaceService;

pub struct ProxyRuntime {
    policy: PolicyEngine,
    workspaces: WorkspaceService,
    checkpoints: CheckpointService,
    changes: ChangeSummaryService,
}

impl ProxyRuntime {
    pub fn new() -> Result<Self> {
        let workspaces = WorkspaceService::new()?;
        Ok(Self {
            policy: PolicyEngine,
            checkpoints: CheckpointService::with_workspaces(workspaces.clone()),
            workspaces,
            changes: ChangeSummaryService,
        })
    }

    pub fn execute(&self, request: ExecutionRequest) -> Result<ExecutionResult> {
        request.validate()?;

        if request.keep_mount {
            return Err(VfsError::InvalidInput(
                "proxy keep-mount is not supported by the runtime path yet; use 'avfs mount' directly".to_string(),
            ));
        }

        let vault_name = self.workspaces.resolve_name(request.vault.as_deref())?;
        let backend = self.workspaces.open(&vault_name)?;
        let decision = self.policy.decide(&request);

        match decision.action {
            PolicyAction::Deny | PolicyAction::RequireApproval => {
                return Err(VfsError::InvalidInput(
                    decision
                        .reason
                        .clone()
                        .unwrap_or_else(|| "proxy policy rejected command".to_string()),
                ));
            }
            PolicyAction::Allow | PolicyAction::AllowWithCheckpoint => {}
        }

        backend.log_operation(
            "proxy_exec_requested",
            None,
            Some(
                &serde_json::json!({
                    "command": request.command_display(),
                    "cwd": request.cwd,
                    "decision": decision.action,
                    "categories": decision.categories,
                })
                .to_string(),
            ),
        )?;

        let baseline = self.changes.baseline(&backend)?;

        let checkpoint = if matches!(decision.action, PolicyAction::AllowWithCheckpoint)
            && matches!(request.checkpoint_mode, CheckpointMode::Auto)
        {
            Some(
                self.checkpoints
                    .create_on_backend(
                        &backend,
                        None,
                        Some(&format!("Auto-checkpoint before: {}", request.command_display())),
                    )?
                    .name,
            )
        } else {
            None
        };

        let mountpoint = request
            .mountpoint
            .clone()
            .unwrap_or_else(|| auto_mountpoint_path(&vault_name));

        let mount_session = MountSession::spawn(
            &vault_name,
            backend.clone(),
            mountpoint.clone(),
            request.readonly,
            false,
            request.mountpoint.is_none(),
        )?;

        let cwd = request.resolve_cwd(mount_session.mountpoint())?;
        std::fs::create_dir_all(&cwd)?;

        let started = Instant::now();
        let output = execute_command(&request, &vault_name, mount_session.mountpoint(), &cwd)?;
        let duration_ms = started.elapsed().as_millis();

        let summary = self.changes.summarize(&backend, baseline)?;

        backend.log_operation(
            "proxy_exec_completed",
            None,
            Some(
                &serde_json::json!({
                    "command": request.command_display(),
                    "exit_code": output.status.code().unwrap_or(-1),
                    "duration_ms": duration_ms,
                    "changed_files": summary.changed_files,
                })
                .to_string(),
            ),
        )?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code().unwrap_or(-1);

        Ok(ExecutionResult {
            vault: vault_name,
            mountpoint: mountpoint.display().to_string(),
            cwd: request.cwd,
            command: request.command_display(),
            exit_code,
            stdout,
            stderr,
            readonly: request.readonly,
            kept_mounted: false,
            duration_ms,
            checkpoint,
            changed_files: summary.changed_files,
            decision,
        })
    }
}

fn execute_command(
    request: &ExecutionRequest,
    vault_name: &str,
    mountpoint: &std::path::Path,
    cwd: &std::path::Path,
) -> Result<std::process::Output> {
    let mut cmd = match &request.command {
        CommandSpec::Argv(argv) => {
            let program = argv.first().ok_or_else(|| {
                VfsError::InvalidInput("proxy requires a command to execute".to_string())
            })?;
            let mut cmd = Command::new(program);
            cmd.args(&argv[1..]);
            cmd
        }
        CommandSpec::Shell(shell) => {
            let shell_bin = std::env::var("SHELL").unwrap_or_else(|_| "bash".to_string());
            let mut cmd = Command::new(shell_bin);
            cmd.arg("-lc").arg(shell);
            cmd
        }
    };

    cmd.current_dir(cwd)
        .env("AVFS_VAULT", vault_name)
        .env("AVFS_MOUNTPOINT", mountpoint.display().to_string())
        .output()
        .map_err(VfsError::Io)
}

fn auto_mountpoint_path(vault: &str) -> PathBuf {
    let pid = std::process::id();
    let timestamp = chrono::Utc::now().timestamp_millis();
    std::env::temp_dir().join(format!("avfs-proxy-{}-{}-{}", vault, pid, timestamp))
}
