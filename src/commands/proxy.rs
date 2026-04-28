//! proxy command - policy-gated execution against a mounted workspace.

use clap::{Args, Subcommand};

use crate::commands::Output;
use crate::error::{Result, VfsError};
use crate::runtime::execution::{
    CheckpointMode, CommandSpec, ExecutionEnvelope, ExecutionRequest, ExecutionTimeout,
};
use crate::runtime::proxy::ProxyRuntime;

#[derive(Args)]
pub struct ProxyArgs {
    #[command(subcommand)]
    pub command: ProxyCommand,
}

#[derive(Subcommand)]
pub enum ProxyCommand {
    /// Execute one top-level command inside a mounted workspace
    Exec(ProxyExecArgs),
}

#[derive(Args)]
pub struct ProxyExecArgs {
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

    /// Timeout in milliseconds (0 disables timeout)
    #[arg(long, default_value = "300000")]
    pub timeout: u64,

    /// Execute a shell command via $SHELL -lc
    #[arg(long, conflicts_with = "command")]
    pub shell: Option<String>,

    /// Command to run inside the mounted workspace
    #[arg(last = true)]
    pub command: Vec<String>,
}

pub fn run(args: ProxyArgs, output: &Output, vault: Option<String>) -> Result<()> {
    match args.command {
        ProxyCommand::Exec(args) => run_exec(args, output, vault),
    }
}

fn run_exec(args: ProxyExecArgs, output: &Output, vault: Option<String>) -> Result<()> {
    let command = match (args.shell, args.command) {
        (Some(shell), argv) if argv.is_empty() => CommandSpec::Shell(shell),
        (None, argv) if !argv.is_empty() => CommandSpec::Argv(argv),
        _ => {
            return Err(VfsError::InvalidInput(
                "proxy exec requires either --shell <command> or -- <command> ...".to_string(),
            ))
        }
    };

    let timeout = if args.timeout == 0 {
        ExecutionTimeout::None
    } else {
        ExecutionTimeout::Millis(args.timeout)
    };

    let request = ExecutionRequest {
        vault,
        cwd: args.cwd,
        readonly: args.readonly,
        keep_mount: args.keep_mount,
        mountpoint: args.mountpoint.map(std::path::PathBuf::from),
        checkpoint_mode: CheckpointMode::Auto,
        command,
        timeout,
    };
    let result = ProxyRuntime::new()?.execute(request.clone())?;

    if output.is_json() {
        output.print_json(&ExecutionEnvelope::new(&request, result.clone()));
    } else {
        if !result.stdout.is_empty() {
            print!("{}", result.stdout);
        }
        if !result.stderr.is_empty() {
            eprint!("{}", result.stderr);
        }
        if result.exit_code != 0 {
            eprintln!("(exit code: {})", result.exit_code);
        }
        if result.timed_out {
            eprintln!("(timed out)");
        }
    }

    if result.exit_code != 0 {
        return Err(VfsError::ExitStatus(result.exit_code));
    }

    Ok(())
}
