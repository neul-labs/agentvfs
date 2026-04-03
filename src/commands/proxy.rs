//! proxy command - policy-gated execution against a mounted workspace.

use clap::Args;

use crate::commands::Output;
use crate::error::{Result, VfsError};
use crate::runtime::execution::{CheckpointMode, CommandSpec, ExecutionRequest};
use crate::runtime::proxy::ProxyRuntime;

#[derive(Args)]
pub struct ProxyArgs {
    /// Mount point to use (auto-generated if omitted)
    #[arg(long)]
    pub mountpoint: Option<String>,

    /// Working directory inside the mounted vault
    #[arg(long, default_value = "/")]
    pub cwd: String,

    /// Mount the vault read-only
    #[arg(long)]
    pub readonly: bool,

    /// Keep the mount active after the command exits
    #[arg(long)]
    pub keep_mount: bool,

    /// Execute a shell command via $SHELL -lc
    #[arg(long, conflicts_with = "command")]
    pub shell: Option<String>,

    /// Command to run inside the mounted workspace
    #[arg(last = true)]
    pub command: Vec<String>,
}

pub fn run(args: ProxyArgs, output: &Output, vault: Option<String>) -> Result<()> {
    let command = match (args.shell, args.command) {
        (Some(shell), argv) if argv.is_empty() => CommandSpec::Shell(shell),
        (None, argv) if !argv.is_empty() => CommandSpec::Argv(argv),
        _ => {
            return Err(VfsError::InvalidInput(
                "proxy requires either --shell <command> or -- <command> ...".to_string(),
            ))
        }
    };

    let request = ExecutionRequest {
        vault,
        cwd: args.cwd,
        readonly: args.readonly,
        keep_mount: args.keep_mount,
        mountpoint: args.mountpoint.map(std::path::PathBuf::from),
        checkpoint_mode: CheckpointMode::Auto,
        command,
    };
    let result = ProxyRuntime::new()?.execute(request)?;

    if output.is_json() {
        output.print_json(&result);
    } else {
        if !result.stdout.is_empty() {
            print!("{}", result.stdout);
        }
        if !result.stderr.is_empty() {
            eprint!("{}", result.stderr);
        }
    }

    if result.exit_code != 0 {
        return Err(VfsError::Internal(format!(
            "proxy command exited with code {}",
            result.exit_code
        )));
    }

    Ok(())
}
