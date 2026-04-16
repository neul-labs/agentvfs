# agentvfs

**Workspace runtime and execution boundary for AI agents**

[![Crates.io](https://img.shields.io/crates/v/agentvfs.svg)](https://crates.io/crates/agentvfs)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Downloads](https://img.shields.io/crates/d/agentvfs.svg)](https://crates.io/crates/agentvfs)

agentvfs is a database-backed virtual filesystem for agent workspaces. The long-term operating model is not just "a fake filesystem", but a proxy boundary that sits between the agent, the shell, and the mounted workspace.

The intended shape is:

```text
agent -> proxy boundary -> mounted forked workspace -> cli tools
```

That boundary is where policy, checkpoints, forks, audit, and command execution should live.

## Outcome View

The target product model is:

- **Vaults** are durable workspace roots
- **Forks** are cheap task workspaces derived from a vault
- **Checkpoints** are rollback points inside a fork
- **Mounts** expose a fork as a real directory for standard tools
- **Proxy execution** is the top-level boundary the agent should use for shell work

The design goal is cheap top-level command control, not syscall-level tracing. agentvfs should see and govern the command the agent requested, prepare the workspace for it, and report the resulting filesystem delta.

## Current Building Blocks

Today the repo already provides the underlying primitives needed for that boundary:

- `vault create`, `vault list`, `vault use`, `vault delete`, `vault info`
- `vault fork` for fast task workspace creation
- `checkpoint ...` as a first-class alias over snapshots
- `mount` / `unmount` for FUSE-backed workspace exposure
- `proxy exec` as the start of a policy-gated execution surface
- `audit`, `quota`, `log`, `diff`, and JSON output for agent integration

The next step is to harden `proxy exec` into the main agent-facing execution surface.

## Quick Start

```bash
# Create a durable workspace root
avfs vault create myproject

# Create some files
avfs mkdir /src
avfs write /src/main.py "print('hello')"
avfs cat /src/main.py

# Create a cheap task workspace
avfs vault fork myproject myproject-task-1 --use

# Save a rollback point before risky work
avfs checkpoint save before-refactor

# Work normally
avfs grep "hello" /
avfs tree /
avfs log /src/main.py
```

## Proxy Boundary Model

The recommended mental model for agent execution is:

1. Agent requests one top-level command.
2. The proxy boundary decides whether that command is allowed.
3. The proxy chooses a vault or fork.
4. The proxy creates a checkpoint if policy requires it.
5. The proxy mounts the workspace.
6. The proxy runs the command in the mounted workspace.
7. The proxy returns stdout, stderr, exit code, and a changed-files summary.

This is intentionally a **top-level command boundary**. It is meant to be cheap and practical. It does not try to trace every subprocess launched from inside scripts.

## Core Commands

| Category | Commands |
|----------|----------|
| **Files** | `ls`, `cat`, `write`, `cp`, `mv`, `rm`, `tree`, `mkdir` |
| **Search** | `grep`, `find`, `search` |
| **Versioning** | `log`, `checkout`, `revert`, `diff` |
| **Metadata** | `tag`, `untag`, `meta` |
| **Import/Export** | `import`, `export`, `exec` |
| **Vaults** | `vault create`, `vault list`, `vault use`, `vault delete`, `vault info`, `vault fork` |
| **Rollback** | `checkpoint save`, `checkpoint restore`, `checkpoint list`, `snapshot ...` |
| **Maintenance** | `stats`, `prune`, `gc`, `compact`, `maintain`, `audit`, `quota` |
| **Shell** | `shell`, `aliases` |
| **FUSE / Runtime** | `mount`, `unmount`, `proxy exec` |

## For AI Agents

Low-level integration today can call the CLI directly:

```python
import json
import subprocess

def avfs(*args):
    result = subprocess.run(
        ["avfs", "--json"] + list(args),
        capture_output=True,
        text=True,
    )
    return json.loads(result.stdout) if result.stdout else None

avfs("vault", "create", "agent-workspace")
avfs("vault", "fork", "agent-workspace", "agent-workspace-task-1")
avfs("--vault", "agent-workspace-task-1", "checkpoint", "save", "before-change")
```

The intended higher-level integration is a proxy boundary that handles:

- command classification
- checkpoint creation
- mount lifecycle
- execution
- change reporting

The current structured proxy path is:

```bash
avfs --json --vault agent-workspace-task-1 proxy exec -- cargo test
```

That JSON output is versioned and intended to stabilize into the agent-facing execution contract.

See [Agent Integration](documentation/advanced/agent-integration.md) and [Proxy Boundary](documentation/advanced/proxy-boundary.md).

## Why agentvfs?

| Feature | Benefit |
|---------|---------|
| **Isolation** | Agent work stays out of the host filesystem |
| **Forking** | New task workspaces can be created cheaply |
| **Checkpoints** | Rollback before risky commands |
| **Auditability** | Commands and file changes can be inspected |
| **Compatibility** | Standard CLI tools can run via mounted workspaces |

## Documentation

- [Quick Start Guide](documentation/getting-started/quickstart.md)
- [Core Concepts](documentation/getting-started/concepts.md)
- [Vault Management](documentation/user-guide/vaults.md)
- [Agent Integration](documentation/advanced/agent-integration.md)
- [Proxy Boundary](documentation/advanced/proxy-boundary.md)
- [FUSE Mount](documentation/advanced/fuse-mount.md)
- [Architecture](documentation/reference/architecture.md)
- [Command Reference](documentation/reference/commands.md)

## License

MIT
