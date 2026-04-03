# Core Concepts

These concepts define the intended operating model for agentvfs.

## Vaults

A **vault** is the durable root of a workspace. It stores files, directories, versions, metadata, tags, snapshots, and audit history.

Use vaults for:

- project roots
- long-lived agent workspaces
- durable state you may want to revisit later

## Forks

A **fork** is a cheap workspace derived from a vault.

Forks are the right unit for:

- one agent task
- one risky experiment
- one isolated command session

The important design point is that agents should usually mutate a fork, not the long-lived root vault directly.

## Checkpoints

A **checkpoint** is a rollback point inside a vault or fork.

Checkpoints are useful when:

- a command may rewrite many files
- the agent is about to run a risky tool
- you want a simple undo point before an experiment

## Mounts

A **mount** exposes a vault or fork as a real directory.

That matters because most developer tools expect:

- normal paths
- normal current working directories
- normal file APIs

Mounts are therefore the bridge between the virtual workspace and ordinary CLI tools.

## Proxy Boundary

The main architectural direction is a **proxy boundary** between the agent and shell execution.

```text
agent -> proxy boundary -> mounted forked workspace -> cli tools
```

The proxy boundary should own:

- top-level command policy
- fork selection
- checkpoint creation
- mount lifecycle
- execution
- audit and change summaries

## Top-Level Command Control

The boundary is intentionally cheap. It should govern the command the agent requested, not every subprocess or syscall underneath it.

That means:

- good at mediating shell work at low cost
- not a deep process sandbox

This is a deliberate trade-off.

## Durable vs Task State

Think in two layers:

- **durable state**
  - vault roots
- **task state**
  - forks and checkpoints

This lets you keep a stable project root while giving each agent task an isolated working area with rollback.

## Recommended Mental Model

1. Create a durable vault for the project.
2. Fork it for a task.
3. Checkpoint before risky work.
4. Mount the fork for tool compatibility.
5. Run the task through the proxy boundary.
6. Inspect, keep, or discard the fork.

## Related Guides

- [Quick Start](quickstart.md)
- [Vault Management](../user-guide/vaults.md)
- [Agent Integration](../advanced/agent-integration.md)
- [Proxy Boundary](../advanced/proxy-boundary.md)
