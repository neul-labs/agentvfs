# agentvfs Roadmap

This roadmap reflects the current strategic direction for agentvfs: a workspace runtime with a cheap proxy boundary between the agent, the shell, and the mounted filesystem.

## Outcome View

The intended operating model is:

```text
agent -> proxy boundary -> mounted forked workspace -> cli tools
```

In that model:

- **vaults** hold durable project state
- **forks** create cheap task workspaces
- **checkpoints** provide rollback inside a task workspace
- **mounts** provide a real directory view for tools
- **proxy execution** becomes the main agent-facing runtime boundary

## Current State

Implemented foundation:

- [x] durable vault storage
- [x] file operations, search, metadata, and versioning
- [x] checkpoints / snapshots
- [x] audit and quota support
- [x] interactive shell and JSON output
- [x] FUSE mount support
- [x] `vault fork`
- [x] initial `proxy` command surface
- [x] runtime service split under `src/runtime/`

The architecture now needs to pivot from "filesystem first, execution second" to "proxy boundary first, filesystem runtime underneath".

## Runtime Refactor Plan

The next phase is no longer just a proposal. The first runtime split is in place, and the remaining work is to harden and extend it without collapsing the abstraction boundaries again.

### Refactor Goals

- move runtime orchestration out of CLI command handlers
- avoid CLI-to-CLI subprocess boundaries inside the proxy path
- keep policy, workspace lifecycle, execution, and storage concerns separate
- make mounts and forks reusable runtime objects rather than ad hoc command behavior
- keep backend-specific storage details below the execution boundary

### Target Runtime Services

The intended internal split is now mostly implemented:

- **PolicyEngine**
  - classify a top-level command
  - return `allow`, `allow_with_checkpoint`, `deny`, or `require_approval`
- **WorkspaceService**
  - resolve current vault
  - create/select task forks
  - manage ephemeral workspace lifecycle
- **CheckpointService**
  - create, restore, list, and delete rollback points
- **MountSession**
  - own mount/unmount lifecycle for a workspace
  - expose a ready mountpoint to the executor
- **ChangeSummary**
  - summarize which files changed after execution
- **ProxyRuntime**
  - orchestrate policy, workspace selection, checkpoints, mounts, execution, and reporting

`CommandExecutor` is still a logical responsibility rather than its own extracted type. Execution is currently performed inside `ProxyRuntime` and can be split later if the boundary starts growing too much.

### Abstraction Rules

The following boundaries should hold:

- CLI command modules should stay thin and mostly parse args, call services, and print output
- `proxy` should use a library-level mount session, not spawn `avfs mount` as a subprocess
- fork lifecycle should not remain mixed into general vault CRUD forever
- policy logic should not live inside storage backends
- changed-file reporting should be a runtime/reporting concern, not a side effect buried in command handlers

### Performance Rules

- classify commands before doing expensive fork or mount work when possible
- keep the proxy path free of avoidable subprocess hops
- make mount readiness explicit rather than inferred from directory existence
- design mount sessions so they can be reused later across multiple top-level commands
- keep fork creation cheap and backend-aware
- avoid global scans for change reporting when a scoped or incremental summary is available

### Staged Refactor Sequence

#### Stage 1. Extract Mount Runtime

- [x] introduce `MountSession` as a library abstraction
- [x] move mount lifecycle out of `proxy` command logic
- [x] remove proxy dependence on spawning `avfs mount`
- [ ] make mount readiness explicit and reusable across longer-lived sessions

#### Stage 2. Extract Execution Types

- [x] define `ExecutionRequest`
- [x] define `ExecutionDecision`
- [x] define `ExecutionResult`
- [ ] finalize a stable external JSON contract for proxy execution

#### Stage 3. Split Workspace Lifecycle from Vault CRUD

- [x] introduce `WorkspaceService`
- [x] move task-fork lifecycle out of general vault management
- [ ] support ephemeral fork creation and cleanup

#### Stage 4. Isolate Policy

- [x] introduce `PolicyEngine` with no filesystem side effects
- [x] move command classification out of CLI handlers
- [x] connect policy decisions to checkpoint behavior

#### Stage 5. Build ProxyRuntime

- [x] orchestrate policy, workspace selection, checkpoints, mounts, execution, and summaries in one library runtime
- [x] keep `proxy` CLI as a thin adapter over `ProxyRuntime`
- [ ] prepare for later session reuse and richer reporting

## Near-Term Priority

### 1. Make Proxy the Main Execution Surface

Goal:

- turn `proxy` into the top-level shell execution boundary for agents

Work:

- [ ] define `proxy exec` as the main entrypoint rather than an initial proxy surface
- [x] accept both argv-style and shell-style command input
- [x] return structured execution results
- [x] make the proxy responsible for mount lifecycle

### 2. Add Top-Level Command Policy

Goal:

- classify and decide on the command the agent requested

Work:

- [x] classify commands as read-only, mutating, destructive, networked, or host-escape-risk
- [x] support `allow`, `allow_with_checkpoint`, `deny`, and `require_approval`
- [x] record policy decisions in audit output

### 3. Treat Forks as Task Workspaces

Goal:

- make forks the default unit of agent work

Work:

- [ ] support ephemeral per-task forks
- [ ] add better lifecycle for short-lived forks
- [ ] improve visibility into fork ancestry and usage

### 4. Make Checkpoints a First-Class Recovery Tool

Goal:

- create rollback points automatically around risky commands

Work:

- [x] auto-checkpoint before mutating or risky proxy executions
- [ ] add clearer restore and discard flows
- [ ] surface checkpoints in agent-oriented output

### 5. Summarize What Changed

Goal:

- make every proxy execution easy to inspect

Work:

- [x] changed-files summary after execution
- [ ] better diff integration for top-level command runs
- [x] execution duration and exit metadata in structured output

## Medium-Term Work

### Storage and Runtime

- [ ] backend migration support
- [ ] improved fork performance and lifecycle management
- [ ] stronger runtime cleanup for stale mounts and abandoned workspaces

### Agent Integration

- [ ] define a stable JSON contract for proxy execution
- [ ] document approval and policy workflows
- [ ] make direct low-level CLI orchestration optional rather than the default agent model

### Developer Experience

- [ ] tighten command reference around shipped behavior
- [ ] add focused docs for proxy workflows
- [ ] improve examples for fork + checkpoint + proxy operation

## Explicit Non-Goals for This Phase

These are not the current target:

- full syscall tracing
- complete subprocess visibility
- replacing kernel-level sandboxing
- multi-tenant permission systems

The objective is the cheapest useful top-level command boundary, not a deep execution monitor.

## Success Criteria

The pivot is successful when:

- an agent asks the proxy to run one top-level command
- the proxy chooses or creates the right workspace fork
- the proxy checkpoints when needed
- the proxy mounts the workspace automatically
- the proxy runs the command
- the proxy returns output plus a useful changed-files summary

At that point, the agent no longer needs to manually orchestrate forks, checkpoints, mounts, and shell invocation.
