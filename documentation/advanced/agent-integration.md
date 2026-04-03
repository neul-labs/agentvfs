# Agent Integration

This guide explains how agentvfs should fit into an agent runtime.

## Preferred Model

The preferred agent model is not direct shell access. It is a mediated execution boundary:

```text
agent -> proxy boundary -> mounted forked workspace -> cli tools
```

That boundary should control:

- which workspace the command runs in
- whether the command is allowed
- whether a checkpoint should be taken first
- how the workspace is mounted
- what changed after execution

## Why This Model

This gives the agent runtime one cheap control point for shell work.

The proxy boundary is intended to govern the **top-level command** requested by the agent. It is not meant to trace every subprocess or syscall.

That trade-off keeps the model practical:

- cheap enough to use per command
- strong enough to prepare and isolate workspaces
- clear enough to audit what the agent asked to do

## Runtime Pieces

The boundary is built from four lower-level primitives already present in the project:

- **Vaults** for durable state
- **Forks** for task-scoped workspaces
- **Checkpoints** for rollback
- **Mounts** for tool compatibility

## Current Low-Level Integration

Today, low-level integrations can still compose the runtime directly:

```python
import json
import subprocess

def avfs(*args):
    result = subprocess.run(
        ["avfs", "--json"] + [str(a) for a in args],
        capture_output=True,
        text=True,
    )
    return json.loads(result.stdout) if result.stdout else None

avfs("vault", "create", "agent-workspace")
avfs("vault", "fork", "agent-workspace", "agent-workspace-task-1")
avfs("--vault", "agent-workspace-task-1", "checkpoint", "save", "before-change")
```

This works, but it is not the desired long-term interface because the agent has to manually orchestrate:

- workspace choice
- fork creation
- checkpoint creation
- mount lifecycle

## Intended Proxy Responsibilities

The proxy boundary should eventually own this lifecycle:

1. receive one top-level command from the agent
2. resolve the target vault or fork
3. classify the command
4. allow, deny, or checkpoint it
5. mount the workspace
6. execute the command
7. report stdout, stderr, exit code, and changed files

## Policy Scope

The proxy policy layer should stay coarse and obvious.

Useful classifications:

- `read_only`
- `mutating`
- `destructive`
- `networked`
- `host_escape_risk`

Useful outcomes:

- `allow`
- `allow_with_checkpoint`
- `deny`
- `require_approval`

## Recommended Workspace Pattern

For agent tasks, use this pattern:

1. keep a durable project root in a vault
2. create a fork for each task
3. save a checkpoint before risky mutations
4. run the task in the fork
5. inspect the resulting changes
6. keep or discard the fork

## What the Proxy Should Return

The ideal top-level execution result includes:

- policy decision
- workspace or fork used
- checkpoint created
- mountpoint used
- exit code
- stdout
- stderr
- changed-files summary

## Non-Goals

The proxy boundary is not supposed to:

- trace every child process launched from a script
- act as a kernel sandbox
- implement syscall-level enforcement

If those requirements appear later, they should be handled below the proxy boundary, not mixed into the first cheap control layer.

## Related Guides

- [Proxy Boundary](proxy-boundary.md)
- [FUSE Mount](fuse-mount.md)
- [Vault Management](../user-guide/vaults.md)
- [Architecture](../reference/architecture.md)
