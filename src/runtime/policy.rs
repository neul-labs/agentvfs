//! Top-level command classification and policy decisions.

use std::collections::BTreeSet;

use crate::runtime::execution::{
    CommandCategory, CommandSpec, ExecutionDecision, ExecutionRequest, PolicyAction,
};

pub struct PolicyEngine;

impl PolicyEngine {
    pub fn decide(&self, request: &ExecutionRequest) -> ExecutionDecision {
        let tokens = tokenize(&request.command);
        let mut categories = BTreeSet::new();

        let program = tokens.first().map(|s| s.as_str()).unwrap_or_default();

        if is_host_escape_risk(&tokens) {
            categories.insert(CommandCategory::HostEscapeRisk);
        }

        if is_networked(program, &tokens) {
            categories.insert(CommandCategory::Networked);
        }

        if is_interactive(program, &tokens) {
            categories.insert(CommandCategory::Interactive);
        }

        if is_destructive(program, &tokens) {
            categories.insert(CommandCategory::Destructive);
        }

        if is_read_only(program, &tokens) {
            categories.insert(CommandCategory::ReadOnly);
        } else if !tokens.is_empty() {
            categories.insert(CommandCategory::Mutating);
        }

        if categories.contains(&CommandCategory::HostEscapeRisk) {
            return ExecutionDecision {
                action: PolicyAction::Deny,
                categories: categories.into_iter().collect(),
                reason: Some(
                    "command references likely host paths outside the workspace".to_string(),
                ),
            };
        }

        if categories.contains(&CommandCategory::Networked) {
            return ExecutionDecision {
                action: PolicyAction::RequireApproval,
                categories: categories.into_iter().collect(),
                reason: Some("networked commands require explicit approval".to_string()),
            };
        }

        let action = if categories.contains(&CommandCategory::Destructive)
            || categories.contains(&CommandCategory::Mutating)
            || categories.contains(&CommandCategory::Interactive)
        {
            PolicyAction::AllowWithCheckpoint
        } else {
            PolicyAction::Allow
        };

        ExecutionDecision {
            action,
            categories: categories.into_iter().collect(),
            reason: None,
        }
    }
}

fn tokenize(command: &CommandSpec) -> Vec<String> {
    match command {
        CommandSpec::Argv(argv) => argv.clone(),
        CommandSpec::Shell(cmd) => shlex::split(cmd).unwrap_or_else(|| vec![cmd.clone()]),
    }
}

fn is_read_only(program: &str, tokens: &[String]) -> bool {
    matches!(
        program,
        "ls" | "cat" | "grep" | "rg" | "find" | "tree" | "pwd" | "echo" | "git"
    ) && !matches!(
        tokens.get(1).map(String::as_str),
        Some("clone" | "clean" | "reset" | "restore")
    )
}

fn is_destructive(program: &str, tokens: &[String]) -> bool {
    match program {
        "rm" => tokens.iter().any(|t| t.contains("-r") || t.contains("-f")),
        "git" => matches!(tokens.get(1).map(String::as_str), Some("clean" | "reset")),
        _ => false,
    }
}

fn is_networked(program: &str, tokens: &[String]) -> bool {
    if matches!(program, "curl" | "wget") {
        return true;
    }

    if program == "git"
        && matches!(
            tokens.get(1).map(String::as_str),
            Some("clone" | "fetch" | "pull")
        )
    {
        return true;
    }

    if matches!(program, "npm" | "pnpm" | "yarn")
        && matches!(tokens.get(1).map(String::as_str), Some("install" | "add"))
    {
        return true;
    }

    if matches!(program, "cargo")
        && matches!(tokens.get(1).map(String::as_str), Some("install" | "add"))
    {
        return true;
    }

    false
}

fn is_host_escape_risk(tokens: &[String]) -> bool {
    tokens.iter().any(|token| {
        token.starts_with("~/")
            || token.starts_with("/etc/")
            || token.starts_with("/Users/")
            || token.starts_with("/home/")
            || token.starts_with("/var/")
            || token.starts_with("/private/")
    })
}

fn is_interactive(program: &str, tokens: &[String]) -> bool {
    matches!(program, "bash" | "sh" | "zsh" | "fish") && tokens.len() <= 1
}

#[cfg(test)]
mod tests {
    use super::PolicyEngine;
    use crate::runtime::execution::{CheckpointMode, CommandSpec, ExecutionRequest, PolicyAction};

    fn argv(parts: &[&str]) -> ExecutionRequest {
        use crate::runtime::execution::ExecutionTimeout;
        ExecutionRequest {
            vault: Some("test".to_string()),
            cwd: "/".to_string(),
            readonly: false,
            keep_mount: false,
            mountpoint: None,
            checkpoint_mode: CheckpointMode::Auto,
            command: CommandSpec::Argv(parts.iter().map(|s| s.to_string()).collect()),
            timeout: ExecutionTimeout::Millis(300_000),
        }
    }

    #[test]
    fn read_only_commands_are_allowed() {
        let decision = PolicyEngine.decide(&argv(&["ls", "/"]));
        assert!(matches!(decision.action, PolicyAction::Allow));
    }

    #[test]
    fn mutating_commands_checkpoint() {
        let decision = PolicyEngine.decide(&argv(&["cargo", "test"]));
        assert!(matches!(decision.action, PolicyAction::AllowWithCheckpoint));
    }

    #[test]
    fn networked_commands_require_approval() {
        let decision = PolicyEngine.decide(&argv(&["curl", "https://example.com"]));
        assert!(matches!(decision.action, PolicyAction::RequireApproval));
    }

    #[test]
    fn host_escape_is_denied() {
        let decision = PolicyEngine.decide(&argv(&["cat", "/etc/passwd"]));
        assert!(matches!(decision.action, PolicyAction::Deny));
    }
}
