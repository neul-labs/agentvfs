//! Proxy runtime orchestration.

use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

use crate::error::{Result, VfsError};
use crate::runtime::change_summary::ChangeSummaryService;
use crate::runtime::checkpoint::CheckpointService;
use crate::runtime::execution::{
    CheckpointMode, CommandSpec, ExecutionRequest, ExecutionResult, ExecutionTimeout, PolicyAction,
    ProxyExecutionState,
};
use crate::runtime::mount_session::MountSession;
use crate::runtime::policy::PolicyEngine;
use crate::runtime::workspace::WorkspaceService;

/// Tracks the execution state and acquired resources for guaranteed cleanup.
struct ExecutionContext {
    state: ProxyExecutionState,
    vault_name: Option<String>,
    mountpoint: Option<PathBuf>,
    checkpoint: Option<String>,
    backend: Option<std::sync::Arc<crate::storage::VaultBackend>>,
}

impl ExecutionContext {
    fn new() -> Self {
        Self {
            state: ProxyExecutionState::Initial,
            vault_name: None,
            mountpoint: None,
            checkpoint: None,
            backend: None,
        }
    }

    fn transition(
        &mut self,
        next: ProxyExecutionState,
    ) -> Result<()> {
        if !self.state.can_transition_to(next) {
            return Err(VfsError::Internal(format!(
                "invalid proxy state transition from {:?} to {:?}",
                self.state, next
            )));
        }
        self.state = next;
        Ok(())
    }
}

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

    pub fn execute(&self,
        request: ExecutionRequest,
    ) -> Result<ExecutionResult> {
        let mut ctx = ExecutionContext::new();
        let result = self.execute_inner(request, &mut ctx);

        match result {
            Ok(r) => {
                let _ = ctx.transition(ProxyExecutionState::Completed);
                Ok(r)
            }
            Err(e) => {
                let _ = ctx.transition(ProxyExecutionState::CleaningUp);
                // MountSession is dropped here, ensuring unmount.
                // Checkpoint is intentionally preserved for forensics.
                let _ = ctx.transition(ProxyExecutionState::Failed);
                Err(e)
            }
        }
    }

    fn execute_inner(
        &self,
        request: ExecutionRequest,
        ctx: &mut ExecutionContext,
    ) -> Result<ExecutionResult> {
        ctx.transition(ProxyExecutionState::Validating)?;
        request.validate()?;

        if request.keep_mount {
            return Err(VfsError::InvalidInput(
                "proxy keep-mount is not supported by the runtime path yet; use 'avfs mount' directly".to_string(),
            ));
        }

        ctx.transition(ProxyExecutionState::Resolving)?;
        let vault_name = self.workspaces.resolve_name(request.vault.as_deref())?;
        ctx.vault_name = Some(vault_name.clone());

        let backend = self.workspaces.open(&vault_name)?;
        ctx.backend = Some(backend.clone());

        ctx.transition(ProxyExecutionState::PolicyEvaluating)?;
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

        ctx.transition(ProxyExecutionState::BaselineCapturing)?;
        let baseline = self.changes.baseline(&backend)?;

        let checkpoint = if matches!(decision.action, PolicyAction::AllowWithCheckpoint)
            && matches!(request.checkpoint_mode, CheckpointMode::Auto)
        {
            ctx.transition(ProxyExecutionState::Checkpointing)?;
            Some(
                self.checkpoints
                    .create_on_backend(
                        &backend,
                        None,
                        Some(&format!(
                            "Auto-checkpoint before: {}",
                            request.command_display()
                        )),
                    )?
                    .name,
            )
        } else {
            None
        };
        ctx.checkpoint = checkpoint.clone();

        let mountpoint = request
            .mountpoint
            .clone()
            .unwrap_or_else(|| auto_mountpoint_path(&vault_name));
        ctx.mountpoint = Some(mountpoint.clone());

        ctx.transition(ProxyExecutionState::Mounting)?;
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

        ctx.transition(ProxyExecutionState::Executing)?;
        let started = Instant::now();
        let (output, timed_out) = execute_command_with_timeout(
            &request,
            &vault_name,
            mount_session.mountpoint(),
            &cwd,
            request.timeout,
        )?;
        let duration_ms = started.elapsed().as_millis();

        // Explicitly drop the mount session to ensure unmount before summarizing
        drop(mount_session);

        ctx.transition(ProxyExecutionState::Summarizing)?;
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
                    "timed_out": timed_out,
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
            state: ProxyExecutionState::Completed,
            timed_out,
        })
    }
}

/// Execute the command with optional timeout.
///
/// Returns the process output and a boolean indicating whether a timeout
/// occurred (in which case the process was killed).
fn execute_command_with_timeout(
    request: &ExecutionRequest,
    vault_name: &str,
    mountpoint: &std::path::Path,
    cwd: &std::path::Path,
    timeout: ExecutionTimeout,
) -> Result<(std::process::Output, bool)> {
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
        .env("AVFS_MOUNTPOINT", mountpoint.display().to_string());

    match timeout {
        ExecutionTimeout::None => {
            let output = cmd.output().map_err(VfsError::Io)?;
            Ok((output, false))
        }
        ExecutionTimeout::Millis(ms) => {
            cmd.stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped());

            let mut child = cmd.spawn().map_err(VfsError::Io)?;

            let timeout = Duration::from_millis(ms);
            let start = Instant::now();

            // Spawn threads to drain stdout/stderr so pipes don't block
            let stdout_handle = child.stdout.take().map(|mut out| {
                std::thread::spawn(move || {
                    let mut buf = Vec::new();
                    std::io::Read::read_to_end(&mut out, &mut buf).ok();
                    buf
                })
            });
            let stderr_handle = child.stderr.take().map(|mut err| {
                std::thread::spawn(move || {
                    let mut buf = Vec::new();
                    std::io::Read::read_to_end(&mut err, &mut buf).ok();
                    buf
                })
            });

            let status = loop {
                match child.try_wait().map_err(VfsError::Io)? {
                    Some(status) => break status,
                    None => {
                        if start.elapsed() >= timeout {
                            let _ = child.kill();
                            let _ = child.wait();
                            return Err(VfsError::InvalidInput(format!(
                                "proxy execution timed out after {}ms",
                                ms
                            )));
                        }
                        std::thread::sleep(Duration::from_millis(10));
                    }
                }
            };

            let stdout = stdout_handle
                .map(|h| h.join().unwrap_or_default())
                .unwrap_or_default();
            let stderr = stderr_handle
                .map(|h| h.join().unwrap_or_default())
                .unwrap_or_default();

            Ok((
                std::process::Output {
                    status,
                    stdout,
                    stderr,
                },
                false,
            ))
        }
    }
}

fn auto_mountpoint_path(vault: &str) -> PathBuf {
    let pid = std::process::id();
    let timestamp = chrono::Utc::now().timestamp_millis();
    std::env::temp_dir().join(format!("avfs-proxy-{}-{}-{}", vault, pid, timestamp))
}
