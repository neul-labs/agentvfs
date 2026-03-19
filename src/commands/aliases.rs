//! aliases command - generate shell aliases.

use clap::{Args, ValueEnum};
use serde::Serialize;

use crate::commands::Output;
use crate::error::Result;

/// Shell type for alias generation.
#[derive(Debug, Clone, Copy, ValueEnum, Default)]
pub enum ShellType {
    #[default]
    Bash,
    Zsh,
    Fish,
}

#[derive(Args)]
pub struct AliasesArgs {
    /// Shell type for alias syntax
    #[arg(long, short, value_enum, default_value = "bash")]
    pub shell: ShellType,

    /// Prefix for alias names (default: v)
    #[arg(long, short, default_value = "v")]
    pub prefix: String,
}

/// List of commands to create aliases for.
const ALIAS_COMMANDS: &[&str] = &[
    "ls", "cat", "write", "mkdir", "rm", "cp", "mv", "tree", "pwd",
    "log", "checkout", "revert", "diff",
    "search", "grep", "find",
    "tag", "untag", "meta",
    "import", "export", "exec",
    "stats", "prune", "gc", "compact", "maintain",
    "quota", "audit", "snapshot",
    "vault", "shell",
];

#[derive(Serialize)]
struct AliasesOutput {
    shell: String,
    prefix: String,
    aliases: Vec<AliasEntry>,
}

#[derive(Serialize)]
struct AliasEntry {
    name: String,
    command: String,
}

pub fn run(args: AliasesArgs, output: &Output) -> Result<()> {
    let prefix = &args.prefix;

    if output.is_json() {
        let aliases: Vec<AliasEntry> = ALIAS_COMMANDS
            .iter()
            .map(|cmd| AliasEntry {
                name: format!("{}{}", prefix, cmd),
                command: format!("vfs {}", cmd),
            })
            .collect();

        output.print_json(&AliasesOutput {
            shell: format!("{:?}", args.shell).to_lowercase(),
            prefix: prefix.clone(),
            aliases,
        });
    } else {
        // Print comment header
        let comment = match args.shell {
            ShellType::Bash | ShellType::Zsh => "#",
            ShellType::Fish => "#",
        };

        println!("{} vfs aliases - add to your shell config", comment);
        println!("{} Generated with: vfs aliases --shell {:?} --prefix {}",
            comment, args.shell, prefix);
        println!();

        // Print aliases
        for cmd in ALIAS_COMMANDS {
            let alias_name = format!("{}{}", prefix, cmd);
            let alias_cmd = format!("vfs {}", cmd);

            match args.shell {
                ShellType::Bash | ShellType::Zsh => {
                    println!("alias {}='{}'", alias_name, alias_cmd);
                }
                ShellType::Fish => {
                    println!("alias {} '{}'", alias_name, alias_cmd);
                }
            }
        }

        println!();
        println!("{} Usage: eval \"$(vfs aliases)\"", comment);
    }

    Ok(())
}
