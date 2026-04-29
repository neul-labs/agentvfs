# Architecture Overview

This document describes the current architecture of agentvfs and the abstraction boundaries the project is converging around.

## Outcome View

agentvfs is no longer just "a virtual filesystem CLI". The intended operating model is:

```text
agent -> proxy boundary -> mounted forked workspace -> cli tools
```

That means the primary product surface is a mediated execution boundary. The filesystem, forks, checkpoints, and mounts sit behind that boundary as runtime mechanics.

## Layered Model

```text
+---------------------------------------------------------------+
|                           Agent Layer                         |
|                top-level command requests only                |
+-------------------------------+-------------------------------+
                                |
+-------------------------------v-------------------------------+
|                      Proxy Boundary Layer                     |
|  ExecutionRequest | PolicyEngine | ProxyRuntime | audit/log   |
+-------------------------------+-------------------------------+
                                |
+-------------------------------v-------------------------------+
|                      Workspace Runtime Layer                  |
| WorkspaceService | CheckpointService | MountSession | changes |
+-------------------------------+-------------------------------+
                                |
+-------------------------------v-------------------------------+
|                         Storage Layer                         |
|     files | versions | metadata | tags | snapshots | audit    |
+-------------------------------+-------------------------------+
                                |
+-------------------------------v-------------------------------+
|                      Storage Backend Layer                    |
|                    SQLite | Sled | LMDB | future              |
+---------------------------------------------------------------+
```

## Core Runtime Abstractions

The current runtime split is implemented under `src/runtime/`.

| Abstraction | Role | Current module |
|-------------|------|----------------|
| `ExecutionRequest` | Normalized top-level command request passed into the proxy runtime | `src/runtime/execution.rs` |
| `ExecutionDecision` | Policy outcome for that request | `src/runtime/execution.rs` |
| `ExecutionResult` | Structured result returned after execution | `src/runtime/execution.rs` |
| `ExecutionEnvelope` | Versioned external proxy-execution contract for JSON output | `src/runtime/execution.rs` |
| `PolicyEngine` | Side-effect-free command classification and decision logic | `src/runtime/policy.rs` |
| `WorkspaceService` | Resolve vaults, describe workspaces, open backends, create forks | `src/runtime/workspace.rs` |
| `CheckpointService` | Create rollback points for risky work | `src/runtime/checkpoint.rs` |
| `ChangeSummaryService` | Summarize changed files after execution | `src/runtime/change_summary.rs` |
| `MountSession` | Own mount lifecycle for a workspace view | `src/runtime/mount_session.rs` |
| `ProxyRuntime` | Orchestrate policy, workspace resolution, checkpoints, mounts, execution, and reporting | `src/runtime/proxy.rs` |

These are the main seams the rest of the system should build on.

## Request / Decision / Result Contracts

The proxy boundary is intentionally explicit about its input and output contracts.

### `CommandSpec`

`CommandSpec` describes how a top-level command is represented:

- `Argv(Vec<String>)`
- `Shell(String)`

The runtime should prefer argv-style execution when available and only fall back to shell-mode when the agent truly needs shell parsing.

### `ExecutionRequest`

`ExecutionRequest` is the canonical input to the proxy runtime. It currently carries:

- target vault selection
- mounted working directory inside the workspace
- read-only intent
- checkpoint mode
- explicit or automatic mountpoint
- the requested top-level command

The request is validated before any expensive runtime work happens.

### `ExecutionDecision`

`ExecutionDecision` is the policy output. It includes:

- `PolicyAction`
  - `Allow`
  - `AllowWithCheckpoint`
  - `Deny`
  - `RequireApproval`
- command categories
- an optional reason

This keeps policy inspectable and easy to audit.

### `ExecutionResult`

`ExecutionResult` is the structured proxy response. It includes:

- vault and mountpoint used
- cwd and command string
- stdout, stderr, and exit code
- execution duration
- whether execution was read-only
- checkpoint created, if any
- changed-files summary
- the policy decision that governed execution

That result shape is the basis for a stable agent-facing JSON contract.

### `ExecutionEnvelope`

`ExecutionEnvelope` is the current versioned JSON wrapper for `proxy exec`. It exists so the external machine-readable contract can evolve deliberately without forcing agents to parse ad hoc CLI output.

It currently includes:

- `schema_version`
- `kind`
- `success`
- normalized request metadata
- the structured `ExecutionResult`

## Policy Boundary

`PolicyEngine` is intentionally a cheap, top-level decision layer.

It classifies a requested command into coarse categories:

- `ReadOnly`
- `Mutating`
- `Destructive`
- `Networked`
- `HostEscapeRisk`
- `Interactive`

The important constraint is that this is a top-level command boundary only. The proxy is not trying to observe every child process or syscall launched from inside scripts.

This is a deliberate performance tradeoff:

- cheap enough to run for every agent command
- useful enough to gate risky work
- simple enough to extend without turning the runtime into a full sandbox

## Workspace Runtime

The workspace runtime owns the mechanics behind the proxy boundary.

### `WorkspaceService`

`WorkspaceService` separates task-workspace lifecycle from CLI argument parsing and from low-level storage code. Its responsibilities are:

- resolve the current vault or an explicitly requested vault
- describe a workspace and open its backend
- create cheap forks for task-scoped work

Fork creation is backend-aware and uses copy-on-write cloning where the host filesystem supports it.

### `CheckpointService`

`CheckpointService` owns checkpoint creation for rollback-oriented workflows. This keeps checkpoint behavior out of command handlers and makes it available to the proxy runtime as a reusable service.

### `MountSession`

`MountSession` is the library-level mount abstraction used by the runtime. It exists to prevent the proxy from shelling out to `avfs mount` as a subprocess.

Its responsibilities are:

- prepare a mountpoint
- create a mounted workspace view
- own cleanup on drop

This is the main performance-oriented seam for future session reuse.

### `ChangeSummaryService`

`ChangeSummaryService` provides a post-execution summary of which workspace paths changed. The current implementation is audit-delta based, which keeps it cheap and scoped to the executed command.

## Proxy Runtime

`ProxyRuntime` is the orchestration layer behind the `proxy` command path.

Its current execution flow is:

1. validate the `ExecutionRequest`
2. resolve the target workspace
3. evaluate policy with `PolicyEngine`
4. reject denied or approval-gated commands
5. baseline change tracking
6. create an automatic checkpoint when policy and mode require it
7. create a `MountSession`
8. execute the top-level command in the mounted workspace
9. summarize changed files
10. return an `ExecutionResult`

This is the right architectural center for future proxy features.

## CLI Adapters

CLI command handlers under `src/commands/` should stay thin.

Their role is:

- parse user input
- construct runtime requests
- call runtime services
- render output

They should not become the place where mount lifecycle, policy decisions, checkpoint strategy, or workspace orchestration accumulates.

## Storage Layer

The storage layer remains responsible for durable filesystem state:

- path resolution
- content storage
- version history
- metadata and tags
- snapshots / checkpoints
- audit records

This layer should stay backend-agnostic. Proxy policy and runtime lifecycle should sit above it, not inside it.

## Backend Layer

Backends provide physical persistence and indexing behavior.

Current direction:

- SQLite as the default durable backend
- Sled as an optional embedded backend
- LMDB as an optional read-heavy backend

The runtime layer should depend on backend capabilities through storage abstractions, not through backend-specific policy logic.

## Extension Rules

These rules matter for both performance and maintainability:

- Keep policy evaluation side-effect-free.
- Classify commands before forks, checkpoints, or mounts when possible.
- Keep mount lifecycle in `MountSession`, not in CLI subprocess wrappers.
- Keep task-workspace lifecycle in `WorkspaceService`, not mixed into general vault CRUD forever.
- Keep changed-file reporting in runtime/reporting services, not buried in command handlers.
- Keep agent-facing execution contracts explicit through `ExecutionRequest`, `ExecutionResult`, and `ExecutionEnvelope`.

## Current Limits

The architecture is intentionally constrained in a few ways right now:

- the boundary governs top-level commands, not child-process tracing
- persistent `keep_mount` proxy sessions are not implemented in the runtime path yet
- `MountSession` and `ProxyRuntime` are currently FUSE-gated
- command execution is orchestrated inside `ProxyRuntime` today and can later be split into a dedicated executor if needed

Those are acceptable limits for the current phase because the main goal is a cheap and practical execution boundary.

## Related Documents

- [Proxy Boundary](../advanced/proxy-boundary.md)
- [Agent Integration](../advanced/agent-integration.md)
- [FUSE Mount](../advanced/fuse-mount.md)
- [Vault Management](../user-guide/vaults.md)
