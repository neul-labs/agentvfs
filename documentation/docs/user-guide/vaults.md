# Vault Management

Vaults are the durable workspace roots in agentvfs.

The intended model is:

- a project has a long-lived vault
- each task gets a cheap fork of that vault
- risky work inside that fork is protected by checkpoints

## What a Vault Contains

A vault stores:

- files and directories
- version history
- metadata and tags
- checkpoints and snapshots
- audit history

By default vaults live under:

```text
~/.avfs/
├── current
└── vaults/
    ├── myproject.avfs
    └── experiments.avfs
```

## Create a Vault

```bash
avfs vault create myproject
```

Create with a specific backend:

```bash
avfs vault create myproject --backend sqlite
```

## List and Select Vaults

```bash
avfs vault list
avfs vault use myproject
avfs vault info myproject
```

Use `--vault` with other commands to target a vault without switching the global current selection.

```bash
avfs --vault myproject ls /
```

## Fork a Vault

Forks are the key workspace primitive for agent tasks.

```bash
avfs vault fork myproject myproject-task-1 --use
```

This creates a new workspace derived from `myproject` and switches to it.

### Why Forks Matter

Forks let you:

- isolate one task from another
- keep a durable project root clean
- create short-lived workspaces cheaply
- checkpoint and discard risky experiments safely

### Recommended Pattern

For agent work, prefer:

1. durable root vault
2. task-specific fork
3. checkpoint inside the fork
4. mounted execution against the fork

## Checkpoints Inside a Vault or Fork

Use checkpoints before risky work:

```bash
avfs checkpoint save before-refactor
avfs checkpoint restore before-refactor
avfs checkpoint list
```

The `snapshot` commands remain available as the underlying implementation surface.

## Delete a Vault

```bash
avfs vault delete old-workspace -y
```

Deletion removes the selected vault backing store. Be careful not to delete durable project roots when you only meant to discard a short-lived task fork.

## Durability and Portability

Vaults are portable because the backing store is local and self-contained.

This makes them useful for:

- backing up workspaces
- moving agent state between machines
- preserving a durable root while throwing away short-lived forks

## How Vaults Relate to the Proxy Boundary

Vaults are not the whole product surface. They are the durable storage layer behind the proxy boundary.

The intended execution path is:

```text
agent -> proxy boundary -> mounted fork -> cli tools
```

From that perspective:

- the vault is the root
- the fork is the task workspace
- the mount is the tool-facing path
- the proxy is the execution boundary

## Related Guides

- [Quick Start](../getting-started/quickstart.md)
- [Proxy Boundary](../advanced/proxy-boundary.md)
- [Agent Integration](../advanced/agent-integration.md)
