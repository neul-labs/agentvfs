# Quick Start

This guide gets you from an empty install to an agent-ready workspace model quickly.

## 1. Create a Durable Workspace Root

A **vault** is the durable root for a project workspace.

```bash
avfs vault create myproject
avfs vault list
```

## 2. Create a Task Workspace

For agent work, the preferred pattern is to create a cheap fork of the main vault and work in that fork.

```bash
avfs vault fork myproject myproject-task-1 --use
```

This gives you an isolated task workspace without mutating the original vault directly.

## 3. Save a Rollback Point

Before risky work, save a checkpoint:

```bash
avfs checkpoint save before-refactor
```

If the task goes wrong, restore it:

```bash
avfs checkpoint restore before-refactor
```

## 4. Work Normally

Inside the fork, use the normal filesystem commands:

```bash
avfs mkdir /src
avfs write /src/main.py "print('hello')"
avfs cat /src/main.py
avfs grep "hello" /
avfs tree /
```

## 5. Inspect Changes

Use versioning and diff tools to understand what changed:

```bash
avfs log /src/main.py
```

Use `audit` and `stats` for a higher-level view:

```bash
avfs audit --limit 20
avfs stats
```

## 6. Use a Real Directory View When Needed

If you need normal CLI tools or editors, mount the workspace:

```bash
mkdir -p /tmp/avfs-mount
avfs mount myproject-task-1 /tmp/avfs-mount --foreground
```

That mounted directory is the runtime substrate for the future proxy boundary.

## 7. Agent Model

The intended long-term model is:

```text
agent -> proxy boundary -> mounted forked workspace -> cli tools
```

Today you can compose that model manually with:

- `vault fork`
- `checkpoint`
- `mount`
- `audit`

The next step is to make `proxy` the main execution surface so the agent asks for one top-level command and the runtime handles workspace preparation automatically.

## Next Steps

- [Core Concepts](concepts.md)
- [Vault Management](../user-guide/vaults.md)
- [Agent Integration](../advanced/agent-integration.md)
- [Proxy Boundary](../advanced/proxy-boundary.md)
- [Architecture](../reference/architecture.md)
