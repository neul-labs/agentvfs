# Architecture Overview

This page mirrors the current agentvfs architecture direction: the system is evolving into a workspace runtime with a proxy boundary between the agent, the shell, and the mounted filesystem.

## Outcome View

The intended operating model is:

```text
agent -> proxy boundary -> mounted forked workspace -> cli tools
```

That means the main abstraction is no longer just a filesystem CLI. It is a mediated execution environment that prepares workspaces, applies policy, runs one top-level command, and reports what changed.

## Main Abstractions

The core runtime abstractions are:

- `ExecutionRequest`
  - normalized top-level command input
- `ExecutionDecision`
  - policy outcome for that request
- `ExecutionResult`
  - structured execution output
- `ExecutionEnvelope`
  - versioned JSON contract for `proxy exec`
- `ExecutionTimeout`
  - configurable timeout with `None` or `Millis(u64)` variants (default: 300s)
- `ProxyExecutionState`
  - explicit FSM for proxy execution lifecycle (`Validating → Mounting → Executing → Completed`)
- `PolicyEngine`
  - cheap classification and decision logic
- `WorkspaceService`
  - vault resolution, backend opening, and task-fork lifecycle
- `CheckpointService`
  - rollback point creation
- `ChangeSummaryService`
  - changed-files summary after execution
- `MountSession`
  - mount/unmount lifecycle for a workspace view with explicit `MountState` transitions
- `ProxyRuntime`
  - orchestration across policy, workspace, checkpoint, mount, execution, and reporting with configurable timeouts

These live under `src/runtime/` and define the main extension seams for the system.

## Layered Model

```text
agent
  -> proxy boundary
  -> workspace runtime
  -> storage layer
  -> storage backend
```

In more detail:

- Agent layer
  - sends one top-level command request
- Proxy boundary
  - applies policy and drives execution
- Workspace runtime
  - resolves vaults, forks, checkpoints, mounts, and change summaries
- Storage layer
  - owns durable filesystem state, versions, metadata, snapshots, and audit
- Storage backend
  - SQLite, Sled, LMDB, and future backends

## Key Architectural Rules

- The proxy is a top-level command boundary, not a syscall monitor.
- CLI command handlers should stay thin and delegate to runtime services.
- Policy should be side-effect-free and evaluated before expensive runtime work.
- Mount lifecycle should stay in `MountSession`, not in CLI-to-CLI subprocess flows.
- Workspace lifecycle should stay separate from general vault CRUD.
- Storage backends should not absorb policy or execution concerns.
- All SQLite mutations are atomic transactions (`BEGIN IMMEDIATE` / `COMMIT` / `ROLLBACK`).
- Proxy execution timeouts escalate from SIGTERM to SIGKILL with dedicated pipe drain threads.
- Open file handles must transition through explicit states to prevent double-persist races.

## Current Limits

- The runtime does not try to observe every subprocess launched from inside scripts.
- FUSE-backed runtime pieces remain feature-gated behind `--features fuse`.
- LMDB and Sled backends do not yet have atomic transaction wrappers (SQLite is recommended for production).

## Reference

For the full architecture write-up, see `documentation/reference/architecture.md`.
