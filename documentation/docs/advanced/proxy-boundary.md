# Proxy Boundary

This document describes the intended control-plane model for agent execution in agentvfs.

## Goal

The key idea is that agent shell execution should cross a single mediated boundary:

```text
agent -> proxy boundary -> mounted workspace fork -> cli tools
```

That boundary should sit between:

- the agent
- the shell entrypoint
- the mounted filesystem view

The proxy is where command policy, workspace preparation, rollback, and execution reporting belong.

## Why a Proxy Boundary

The project is not aiming for a full syscall monitor or deep process sandbox. That would be much heavier.

Instead, the proxy boundary should be cheap and practical:

- it sees the **top-level command** requested by the agent
- it decides whether that command is allowed
- it prepares a workspace for that command
- it runs the command
- it reports what happened

That gives useful control without trying to observe every subprocess launched inside scripts.

## Runtime Pieces

The boundary is built from lower-level primitives:

- **Vaults**: durable workspace roots
- **Forks**: cheap task-scoped copies of a vault
- **Checkpoints**: rollback points inside a fork
- **Mounts**: real directory views for normal tools
- **Audit**: command and filesystem history

These should be treated as runtime mechanics behind the proxy, not as the main interface the agent orchestrates manually.

## Top-Level Execution Flow

The intended proxy execution lifecycle is:

1. Receive a top-level command request from the agent.
2. Resolve the target vault or create/select a fork.
3. Classify the command.
4. Decide `allow`, `allow_with_checkpoint`, `deny`, or `require_approval`.
5. Create a checkpoint when policy requires it.
6. Mount the selected workspace.
7. Run the command in the mounted working directory.
8. Capture stdout, stderr, exit code, and duration.
9. Summarize changed files.
10. Unmount or keep the session alive, depending on policy.

## Policy Scope

The policy layer is intentionally coarse.

It should reason about the top-level command only:

- `read_only`
- `mutating`
- `destructive`
- `networked`
- `host_escape_risk`

Typical outcomes:

- `allow`
- `allow_with_checkpoint`
- `deny`
- `require_approval`

## Non-Goals

The proxy boundary is not meant to:

- trace every subprocess launched from inside scripts
- inspect every syscall
- replace a kernel sandbox
- implement a full multi-tenant security model

If those guarantees become necessary, they should be added below the proxy boundary, not folded into the first cheap control layer.

## Current State

The repository already includes most of the enabling primitives:

- `vault fork`
- `checkpoint`
- `mount` / `unmount`
- `proxy exec`
- JSON output and audit support

What remains is to harden `proxy exec` into the primary agent-facing execution endpoint with stronger session lifecycle and richer reporting.

## Recommended External Shape

The preferred agent-facing interface is a small execution API:

- `proxy exec`
- optional later: `proxy prepare`, `proxy rollback`, `proxy inspect`

The important point is that the agent should not need to manually orchestrate:

- fork creation
- checkpoint creation
- mount lifecycle
- shell invocation

That orchestration belongs behind the proxy boundary.
