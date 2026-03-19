//! REPL (Read-Eval-Print-Loop) for the interactive shell.

use std::path::PathBuf;

use rustyline::error::ReadlineError;
use rustyline::{Config, Editor};

use crate::commands::Output;
use crate::error::{Result, VfsError};
use crate::vault::VaultManager;

use super::completion::ShellHelper;

/// Interactive shell for avfs.
pub struct Shell {
    /// Rustyline editor.
    editor: Editor<ShellHelper, rustyline::history::DefaultHistory>,
    /// Path to history file.
    history_path: Option<PathBuf>,
    /// Whether to persist history.
    persist_history: bool,
}

impl Shell {
    /// Create a new shell instance.
    pub fn new(persist_history: bool) -> Result<Self> {
        // Build config - some methods return Result, others return Self directly
        let config = Config::builder()
            .history_ignore_dups(true)
            .map_err(|e| VfsError::Internal(format!("config error: {}", e)))?
            .history_ignore_space(true)
            .max_history_size(1000)
            .map_err(|e| VfsError::Internal(format!("config error: {}", e)))?
            .build();

        let mut editor = Editor::with_config(config)
            .map_err(|e| VfsError::Internal(format!("failed to create shell editor: {}", e)))?;
        editor.set_helper(Some(ShellHelper::new()));

        let history_path = if persist_history {
            dirs::home_dir().map(|h| h.join(".avfs").join("history"))
        } else {
            None
        };

        // Load history if available
        if let Some(path) = &history_path {
            let _ = editor.load_history(path);
        }

        Ok(Self {
            editor,
            history_path,
            persist_history,
        })
    }

    /// Get the current vault name for the prompt.
    fn get_vault_name(&self) -> String {
        match VaultManager::new() {
            Ok(manager) => match manager.current() {
                Ok(Some(name)) => name,
                _ => "(no vault)".to_string(),
            },
            Err(_) => "(no vault)".to_string(),
        }
    }

    /// Run the interactive shell.
    pub fn run(&mut self) -> Result<()> {
        println!("avfs interactive shell");
        println!("Type 'help' for available commands, 'exit' to quit.\n");

        loop {
            // Update completion backend for path completion
            if let Some(helper) = self.editor.helper_mut() {
                helper.update_backend();
            }

            // Build prompt with vault name
            let vault_name = self.get_vault_name();
            let prompt = format!("{}> ", vault_name);

            // Read line
            let readline = self.editor.readline(&prompt);

            match readline {
                Ok(line) => {
                    let line = line.trim();

                    // Skip empty lines
                    if line.is_empty() {
                        continue;
                    }

                    // Add to history
                    let _ = self.editor.add_history_entry(line);

                    // Handle built-in commands
                    match line {
                        "exit" | "quit" => {
                            println!("Goodbye!");
                            break;
                        }
                        "help" => {
                            self.print_help();
                            continue;
                        }
                        "clear" => {
                            // Clear screen using ANSI escape
                            print!("\x1b[2J\x1b[H");
                            continue;
                        }
                        _ => {}
                    }

                    // Execute command
                    self.execute_command(line);
                }
                Err(ReadlineError::Interrupted) => {
                    // Ctrl-C - print newline and continue
                    println!("^C");
                    continue;
                }
                Err(ReadlineError::Eof) => {
                    // Ctrl-D - exit
                    println!("Goodbye!");
                    break;
                }
                Err(err) => {
                    eprintln!("Error: {}", err);
                    break;
                }
            }
        }

        // Save history
        if self.persist_history {
            if let Some(path) = &self.history_path {
                // Ensure directory exists
                if let Some(parent) = path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                let _ = self.editor.save_history(path);
            }
        }

        Ok(())
    }

    /// Execute a command line.
    fn execute_command(&self, line: &str) {
        // Parse the line into arguments
        let args = match shlex::split(line) {
            Some(args) => args,
            None => {
                eprintln!("Error: invalid command syntax");
                return;
            }
        };

        if args.is_empty() {
            return;
        }

        // Prepend "avfs" to make it a valid CLI invocation
        let full_args: Vec<String> = std::iter::once("avfs".to_string())
            .chain(args.into_iter())
            .collect();

        // Use clap to parse and execute
        self.run_cli_command(&full_args);
    }

    /// Run a CLI command with the given arguments.
    fn run_cli_command(&self, args: &[String]) {
        use clap::Parser;

        // We need to define a local Cli struct that matches main.rs
        // Since we can't easily import from main.rs, we'll use try_parse_from
        // and handle the command execution here

        #[derive(Parser)]
        #[command(name = "avfs")]
        #[command(version, about = "Virtual filesystem CLI backed by embedded databases")]
        struct Cli {
            #[arg(long, global = true)]
            vault: Option<String>,

            #[arg(long, global = true)]
            json: bool,

            #[command(subcommand)]
            command: CliCommands,
        }

        #[derive(clap::Subcommand)]
        enum CliCommands {
            Vault(crate::commands::vault::VaultArgs),
            Ls(crate::commands::ls::LsArgs),
            Cat(crate::commands::cat::CatArgs),
            Write(crate::commands::write::WriteArgs),
            Mkdir(crate::commands::mkdir::MkdirArgs),
            Rm(crate::commands::rm::RmArgs),
            Cp(crate::commands::cp::CpArgs),
            Mv(crate::commands::mv::MvArgs),
            Tree(crate::commands::tree::TreeArgs),
            Log(crate::commands::log::LogArgs),
            Checkout(crate::commands::checkout::CheckoutArgs),
            Revert(crate::commands::revert::RevertArgs),
            Diff(crate::commands::diff::DiffArgs),
            Search(crate::commands::search::SearchArgs),
            Grep(crate::commands::grep::GrepArgs),
            Find(crate::commands::find::FindArgs),
            Tag(crate::commands::tag::TagArgs),
            Untag(crate::commands::untag::UntagArgs),
            Meta(crate::commands::meta::MetaArgs),
            Import(crate::commands::import::ImportArgs),
            Export(crate::commands::export::ExportArgs),
            Exec(crate::commands::exec::ExecArgs),
            Stats(crate::commands::stats::StatsArgs),
            Prune(crate::commands::prune::PruneArgs),
            Gc(crate::commands::gc::GcArgs),
            Compact(crate::commands::compact::CompactArgs),
            Maintain(crate::commands::maintain::MaintainArgs),
            Quota(crate::commands::quota::QuotaArgs),
            Audit(crate::commands::audit::AuditArgs),
            Snapshot(crate::commands::snapshot::SnapshotArgs),
            Pwd,
        }

        // Try to parse the command
        let cli = match Cli::try_parse_from(args) {
            Ok(cli) => cli,
            Err(e) => {
                // Print clap error (help, version, or actual error)
                eprintln!("{}", e);
                return;
            }
        };

        let output = Output::new(cli.json);

        // Execute the command
        let result = match cli.command {
            CliCommands::Vault(args) => crate::commands::vault::run(args, &output),
            CliCommands::Ls(args) => crate::commands::ls::run(args, &output, cli.vault),
            CliCommands::Cat(args) => crate::commands::cat::run(args, &output, cli.vault),
            CliCommands::Write(args) => crate::commands::write::run(args, &output, cli.vault),
            CliCommands::Mkdir(args) => crate::commands::mkdir::run(args, &output, cli.vault),
            CliCommands::Rm(args) => crate::commands::rm::run(args, &output, cli.vault),
            CliCommands::Cp(args) => crate::commands::cp::run(args, &output, cli.vault),
            CliCommands::Mv(args) => crate::commands::mv::run(args, &output, cli.vault),
            CliCommands::Tree(args) => crate::commands::tree::run(args, &output, cli.vault),
            CliCommands::Log(args) => crate::commands::log::run(args, &output, cli.vault),
            CliCommands::Checkout(args) => crate::commands::checkout::run(args, &output, cli.vault),
            CliCommands::Revert(args) => crate::commands::revert::run(args, &output, cli.vault),
            CliCommands::Diff(args) => crate::commands::diff::run(args, &output, cli.vault),
            CliCommands::Search(args) => crate::commands::search::run(args, &output, cli.vault),
            CliCommands::Grep(args) => crate::commands::grep::run(args, &output, cli.vault),
            CliCommands::Find(args) => crate::commands::find::run(args, &output, cli.vault),
            CliCommands::Tag(args) => crate::commands::tag::run(args, &output, cli.vault),
            CliCommands::Untag(args) => crate::commands::untag::run(args, &output, cli.vault),
            CliCommands::Meta(args) => crate::commands::meta::run(args, &output, cli.vault),
            CliCommands::Import(args) => crate::commands::import::run(args, &output, cli.vault),
            CliCommands::Export(args) => crate::commands::export::run(args, &output, cli.vault),
            CliCommands::Exec(args) => crate::commands::exec::run(args, &output, cli.vault),
            CliCommands::Stats(args) => crate::commands::stats::run(args, &output, cli.vault),
            CliCommands::Prune(args) => crate::commands::prune::run(args, &output, cli.vault),
            CliCommands::Gc(args) => crate::commands::gc::run(args, &output, cli.vault),
            CliCommands::Compact(args) => crate::commands::compact::run(args, &output, cli.vault),
            CliCommands::Maintain(args) => crate::commands::maintain::run(args, &output, cli.vault),
            CliCommands::Quota(args) => crate::commands::quota::run(args, &output, cli.vault),
            CliCommands::Audit(args) => crate::commands::audit::run(args, &output, cli.vault),
            CliCommands::Snapshot(args) => crate::commands::snapshot::run(args, &output, cli.vault),
            CliCommands::Pwd => {
                if cli.json {
                    output.print_json(&serde_json::json!({"path": "/"}));
                } else {
                    println!("/");
                }
                Ok(())
            }
        };

        // Handle errors (but don't exit the shell)
        if let Err(e) = result {
            output.print_error(&e);
        }
    }

    /// Print help information.
    fn print_help(&self) {
        println!("Available commands:");
        println!();
        println!("  File Operations:");
        println!("    ls [PATH]          List directory contents");
        println!("    cat <PATH>         Read file contents");
        println!("    write <PATH> TEXT  Write content to file");
        println!("    mkdir <PATH>       Create directory");
        println!("    rm <PATH>          Remove file or directory");
        println!("    cp <SRC> <DST>     Copy file or directory");
        println!("    mv <SRC> <DST>     Move/rename file");
        println!("    tree [PATH]        Display directory tree");
        println!("    pwd                Print working directory");
        println!();
        println!("  Versioning:");
        println!("    log <PATH>         Show version history");
        println!("    checkout <PATH>    Restore specific version");
        println!("    revert <PATH>      Revert to previous version");
        println!("    diff <A> <B>       Compare files or versions");
        println!();
        println!("  Search:");
        println!("    search <QUERY>     Full-text search");
        println!("    grep <PATTERN>     Regex content search");
        println!("    find <NAME>        Find files by name");
        println!();
        println!("  Tags & Metadata:");
        println!("    tag <PATH> [TAG]   Add or list tags");
        println!("    untag <PATH> <TAG> Remove tag");
        println!("    meta <PATH>        Get/set metadata");
        println!();
        println!("  Import/Export:");
        println!("    import <REAL> <VFS>  Import from filesystem");
        println!("    export <VFS> <REAL>  Export to filesystem");
        println!("    exec <CMD> <PATH>    Run command on file");
        println!();
        println!("  Maintenance:");
        println!("    stats              Show storage statistics");
        println!("    prune              Prune old versions");
        println!("    gc                 Garbage collection");
        println!("    compact            Compact database");
        println!("    maintain           Full maintenance");
        println!();
        println!("  Agent Features:");
        println!("    quota              Manage quotas");
        println!("    audit              View audit log");
        println!("    snapshot           Manage snapshots");
        println!();
        println!("  Vault Management:");
        println!("    vault create <N>   Create vault");
        println!("    vault list         List vaults");
        println!("    vault use <N>      Switch vault");
        println!("    vault delete <N>   Delete vault");
        println!("    vault info [N]     Show vault info");
        println!();
        println!("  Shell:");
        println!("    help               Show this help");
        println!("    clear              Clear screen");
        println!("    exit / quit        Exit shell");
        println!();
        println!("Use '--help' after any command for detailed usage.");
    }
}
