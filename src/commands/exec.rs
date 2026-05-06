//! exec command - run external commands on virtual files.

use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};

use clap::Args;
use serde::Serialize;
use tempfile::NamedTempFile;

use crate::commands::Output;
use crate::error::Result;
use crate::fs::FileSystem;
use crate::vault::VaultManager;

#[derive(Args)]
pub struct ExecArgs {
    /// Path in virtual filesystem
    pub vfs_path: String,

    /// Command and arguments (after --)
    #[arg(last = true, required = true)]
    pub command: Vec<String>,

    /// Re-import file after command completes
    #[arg(long)]
    pub reimport: bool,

    /// Pass content via stdin instead of temp file
    #[arg(long)]
    pub stdin: bool,
}

#[derive(Serialize)]
struct ExecOutput {
    vfs_path: String,
    command: String,
    exit_code: i32,
    stdout: String,
    stderr: String,
    reimported: bool,
    reimport_bytes: Option<u64>,
}

pub fn run(args: ExecArgs, output: &Output, vault: Option<String>) -> Result<()> {
    let manager = VaultManager::new()?;
    let backend = match vault {
        Some(name) => manager.open(&name)?,
        None => manager.open_current()?,
    };

    let vfs = FileSystem::new(backend);

    // Read file content
    let content = vfs.read_file(&args.vfs_path)?;

    // Create temp file and write content
    let mut temp_file = NamedTempFile::new().map_err(crate::error::VfsError::Io)?;
    temp_file
        .write_all(&content)
        .map_err(crate::error::VfsError::Io)?;
    temp_file.flush().map_err(crate::error::VfsError::Io)?;

    let temp_path = temp_file.path();

    // Build command
    if args.command.is_empty() {
        return Err(crate::error::VfsError::Internal(
            "no command specified".to_string(),
        ));
    }

    let mut cmd = Command::new(&args.command[0]);

    // Check if {} placeholder is used
    let has_placeholder = args.command.iter().any(|arg| arg.contains("{}"));

    // Add arguments
    for arg in &args.command[1..] {
        if arg.contains("{}") {
            // Replace {} with temp file path
            let replaced = arg.replace("{}", temp_path.to_string_lossy().as_ref());
            cmd.arg(replaced);
        } else {
            cmd.arg(arg);
        }
    }

    // If no placeholder and not using stdin, append temp file path
    if !has_placeholder && !args.stdin {
        cmd.arg(temp_path);
    }

    // Handle stdin mode
    if args.stdin {
        cmd.stdin(Stdio::piped());
    }

    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    // Execute command
    let mut child = cmd.spawn().map_err(crate::error::VfsError::Io)?;

    // If using stdin, write content to it
    if args.stdin {
        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(&content)
                .map_err(crate::error::VfsError::Io)?;
        }
    }

    let cmd_output = child
        .wait_with_output()
        .map_err(crate::error::VfsError::Io)?;

    let exit_code = cmd_output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&cmd_output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&cmd_output.stderr).to_string();

    // Re-import if requested
    let mut reimport_bytes = None;
    if args.reimport {
        let new_content = fs::read(temp_path).map_err(crate::error::VfsError::Io)?;
        reimport_bytes = Some(new_content.len() as u64);
        vfs.write_file(&args.vfs_path, &new_content)?;
    }

    let command_str = args.command.join(" ");

    if output.is_json() {
        output.print_json(&ExecOutput {
            vfs_path: args.vfs_path,
            command: command_str,
            exit_code,
            stdout,
            stderr,
            reimported: args.reimport,
            reimport_bytes,
        });
    } else {
        // Print stdout directly
        if !stdout.is_empty() {
            print!("{}", stdout);
        }
        if !stderr.is_empty() {
            eprint!("{}", stderr);
        }
        if args.reimport {
            if let Some(bytes) = reimport_bytes {
                eprintln!("(reimported {} bytes)", bytes);
            }
        }
        if exit_code != 0 {
            eprintln!("(exit code: {})", exit_code);
        }
    }

    // Return error if command failed
    if exit_code != 0 {
        return Err(crate::error::VfsError::ExitStatus(exit_code));
    }

    Ok(())
}
