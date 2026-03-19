//! Tab completion for the interactive shell.

use std::sync::Arc;

use rustyline::completion::{Completer, Pair};
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::{Context, Helper};

use crate::fs::FileSystem;
use crate::storage::SqliteBackend;
use crate::vault::VaultManager;

/// List of all available commands.
pub const COMMANDS: &[&str] = &[
    "vault", "ls", "cat", "write", "mkdir", "rm", "cp", "mv", "tree", "pwd",
    "log", "checkout", "revert", "diff",
    "search", "grep", "find",
    "tag", "untag", "meta",
    "import", "export", "exec",
    "stats", "prune", "gc", "compact", "maintain",
    "quota", "audit", "snapshot",
    // Built-in shell commands
    "exit", "quit", "help", "clear",
];

/// Subcommands for commands that have them.
pub const SUBCOMMANDS: &[(&str, &[&str])] = &[
    ("vault", &["create", "list", "use", "delete", "info"]),
    ("snapshot", &["save", "list", "restore", "delete", "info"]),
    ("quota", &["set", "clear"]),
    ("audit", &["clear"]),
    ("tag", &["--list", "--create", "--delete", "--rename"]),
];

/// Shell helper that provides completion.
pub struct ShellHelper {
    /// Current vault backend (if any).
    backend: Option<Arc<SqliteBackend>>,
}

impl ShellHelper {
    /// Create a new shell helper.
    pub fn new() -> Self {
        Self { backend: None }
    }

    /// Update the backend reference for path completion.
    pub fn update_backend(&mut self) {
        // Try to get current vault backend
        if let Ok(manager) = VaultManager::new() {
            if let Ok(backend) = manager.open_current() {
                self.backend = Some(backend);
                return;
            }
        }
        self.backend = None;
    }

    /// Complete command names.
    fn complete_command(&self, partial: &str) -> Vec<Pair> {
        COMMANDS
            .iter()
            .filter(|cmd| cmd.starts_with(partial))
            .map(|cmd| Pair {
                display: cmd.to_string(),
                replacement: cmd.to_string(),
            })
            .collect()
    }

    /// Complete subcommands for a given command.
    fn complete_subcommand(&self, cmd: &str, partial: &str) -> Vec<Pair> {
        for (command, subs) in SUBCOMMANDS {
            if *command == cmd {
                return subs
                    .iter()
                    .filter(|sub| sub.starts_with(partial))
                    .map(|sub| Pair {
                        display: sub.to_string(),
                        replacement: sub.to_string(),
                    })
                    .collect();
            }
        }
        Vec::new()
    }

    /// Complete file paths from the virtual filesystem.
    fn complete_path(&self, partial: &str) -> Vec<Pair> {
        let Some(backend) = &self.backend else {
            return Vec::new();
        };

        let fs = FileSystem::new(backend.clone());

        // Determine the parent directory and prefix
        let (parent, prefix) = if partial.is_empty() || partial == "/" {
            ("/", "")
        } else if partial.ends_with('/') {
            (partial, "")
        } else {
            // Split into parent and prefix
            match partial.rfind('/') {
                Some(pos) => {
                    let parent = if pos == 0 { "/" } else { &partial[..pos] };
                    let prefix = &partial[pos + 1..];
                    (parent, prefix)
                }
                None => ("/", partial),
            }
        };

        // List entries in parent directory
        let entries = match fs.list_dir(parent) {
            Ok(entries) => entries,
            Err(_) => return Vec::new(),
        };

        // Filter and format completions
        entries
            .iter()
            .filter(|entry| entry.name.starts_with(prefix))
            .map(|entry| {
                let path = if parent == "/" {
                    format!("/{}", entry.name)
                } else {
                    format!("{}/{}", parent, entry.name)
                };

                // Add trailing slash for directories
                let display = if entry.file_type.is_dir() {
                    format!("{}/", entry.name)
                } else {
                    entry.name.clone()
                };

                let replacement = if entry.file_type.is_dir() {
                    format!("{}/", path)
                } else {
                    path
                };

                Pair {
                    display,
                    replacement,
                }
            })
            .collect()
    }
}

impl Completer for ShellHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        let line = &line[..pos];
        let words: Vec<&str> = line.split_whitespace().collect();

        match words.len() {
            0 => {
                // Empty line - complete command names
                Ok((0, self.complete_command("")))
            }
            1 => {
                // First word - complete command or partial command
                if line.ends_with(' ') {
                    // Command complete, start subcommand or path
                    let cmd = words[0];
                    let subs = self.complete_subcommand(cmd, "");
                    if !subs.is_empty() {
                        Ok((pos, subs))
                    } else {
                        // No subcommands, try path completion
                        Ok((pos, self.complete_path("")))
                    }
                } else {
                    // Still typing command
                    Ok((0, self.complete_command(words[0])))
                }
            }
            _ => {
                // Multiple words
                let cmd = words[0];
                let last = words.last().unwrap();

                if line.ends_with(' ') {
                    // Just finished a word, start new completion
                    if words.len() == 1 {
                        // After command, try subcommand
                        let subs = self.complete_subcommand(cmd, "");
                        if !subs.is_empty() {
                            return Ok((pos, subs));
                        }
                    }
                    // Default to path completion
                    Ok((pos, self.complete_path("")))
                } else {
                    // In the middle of a word
                    let start = line.rfind(' ').map(|i| i + 1).unwrap_or(0);

                    // Check if this is a subcommand position
                    if words.len() == 2 {
                        let subs = self.complete_subcommand(cmd, last);
                        if !subs.is_empty() {
                            return Ok((start, subs));
                        }
                    }

                    // Try path completion if it looks like a path
                    if last.starts_with('/') || last.contains('/') {
                        Ok((start, self.complete_path(last)))
                    } else {
                        // Could be a flag or path - try both
                        let paths = self.complete_path(last);
                        Ok((start, paths))
                    }
                }
            }
        }
    }
}

impl Hinter for ShellHelper {
    type Hint = String;

    fn hint(&self, _line: &str, _pos: usize, _ctx: &Context<'_>) -> Option<String> {
        None
    }
}

impl Highlighter for ShellHelper {}

impl Validator for ShellHelper {}

impl Helper for ShellHelper {}
