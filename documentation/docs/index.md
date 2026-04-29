# agentvfs

**Workspace runtime and execution boundary for AI agents**

agentvfs is a virtual filesystem for agent workspaces, but the intended product boundary is broader than storage alone. The long-term interface is a proxy boundary between the agent, the shell, and a mounted workspace fork.

```text
agent -> proxy boundary -> mounted forked workspace -> cli tools
```

## What agentvfs Is Becoming

The target operating model is:

- **Vaults** hold durable workspace state
- **Forks** create cheap task-scoped workspaces
- **Checkpoints** provide rollback inside a fork
- **Mounts** expose a fork as a real directory
- **Proxy execution** becomes the top-level shell and filesystem mediation layer

This is a top-level command boundary. It is meant to classify, allow, deny, checkpoint, run, and summarize the command the agent requested. It is not a syscall tracer.

## Current State

The repository already contains the runtime pieces needed for that model:

- `vault fork`
- `checkpoint`
- `mount` / `unmount`
- `proxy`
- `audit`
- JSON output and CLI primitives

Those pieces are currently exposed as lower-level commands. The roadmap now shifts toward making the proxy boundary the primary agent-facing execution surface.

## Install

```bash
# Shell installer (recommended)
curl -sSfL https://raw.githubusercontent.com/neul-labs/agentvfs/main/install.sh | bash

# Or pick a package manager:
cargo install agentvfs
npm install -g agentvfs-cli
pip install agentvfs-cli
```

The pip wrapper auto-fetches the native binary lazily on first use. See [Installation](getting-started/installation.md) for all four channels and how they relate.

## Quick Example

```bash
# Create a durable workspace root
avfs vault create myproject

# Create a task workspace
avfs vault fork myproject myproject-task-1 --use

# Save a rollback point
avfs checkpoint save before-refactor

# Work inside the fork
avfs mkdir /src
avfs write /src/main.py "print('hello')"
avfs grep "hello" /
```

## Why This Model

| Goal | Runtime Piece |
|------|---------------|
| Isolate work from the host filesystem | Vaults and mounts |
| Make task setup cheap | Forks |
| Make risky commands recoverable | Checkpoints |
| Run normal tools against a real directory | FUSE mount |
| Control shell work at low cost | Proxy boundary |

## Read Next

- [Quick Start](getting-started/quickstart.md)
- [Core Concepts](getting-started/concepts.md)
- [Vault Management](user-guide/vaults.md)
- [Agent Integration](advanced/agent-integration.md)
- [Python SDK](advanced/python-sdk.md)
- [Proxy Boundary](advanced/proxy-boundary.md)
- [FUSE Mount](advanced/fuse-mount.md)
- [Architecture](reference/architecture.md)
- [Command Reference](reference/commands.md)
