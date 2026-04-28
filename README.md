# agentvfs

<p align="center"><strong>Workspace runtime and execution boundary for AI agents</strong></p>

<p align="center">
  <a href="https://crates.io/crates/agentvfs"><img src="https://img.shields.io/crates/v/agentvfs.svg" alt="Crates.io"></a>
  <a href="https://docs.neullabs.com/agentvfs"><img src="https://img.shields.io/badge/docs-neullabs.com-blue.svg" alt="Documentation"></a>
  <a href="https://opensource.org/licenses/MIT"><img src="https://img.shields.io/badge/License-MIT-yellow.svg" alt="License: MIT"></a>
  <a href="https://crates.io/crates/agentvfs"><img src="https://img.shields.io/crates/d/agentvfs.svg" alt="Downloads"></a>
</p>

---

## What You Get

For **agent developers** and **architects** building autonomous systems:

| Capability | What It Means |
|------------|---------------|
| **Isolated Workspaces** | Agents operate in contained vaults, not your host filesystem |
| **Instant Forks** | Spin up cheap task workspaces in milliseconds |
| **Rollback Points** | Checkpoint before risky operations, restore if needed |
| **Proxy Boundary** | Single execution surface with policy control and change tracking |
| **Standard Tooling** | Mount workspaces as real directories—`git`, `cargo`, `npm` just work |
| **Structured Output** | JSON responses designed for agent integration |

The proxy boundary mediates between your agent and the workspace:

```text
agent → proxy boundary → mounted forked workspace → cli tools
```

It governs what commands run, creates checkpoints, and reports filesystem deltas back to your agent.

## Quick Start

### Install

**With cargo:**
```bash
cargo install agentvfs
```

**With npm:**
```bash
npm install -g @neullabs/agentvfs
```

**With pip:**
```bash
pip install agentvfs
```

### Basic Usage

```bash
# Create a workspace vault
avfs vault create myproject

# Add files
avfs mkdir /src
avfs write /src/main.py "print('hello')"

# Fork for a task
avfs vault fork myproject task-001 --use

# Checkpoint before risky work
avfs checkpoint save before-refactor

# Run commands via proxy (recommended for agents)
avfs proxy exec -- python /src/main.py
```

### Agent Integration (Python)

```python
from agentvfs import AVFS

avfs = AVFS("agent-workspace")

# Create isolated workspace
avfs.create_vault("agent-workspace")
avfs.fork_vault("agent-workspace", "task-001")

# Checkpoint, execute, get structured result
avfs.checkpoint_save("pre-change")
result = avfs.proxy_exec("cargo", "test")
# result contains: stdout, stderr, exit_code, changed_files
```

See the [full documentation](https://docs.neullabs.com/agentvfs) for more integration patterns.

## How It Works

1. **Agent requests a command** → `avfs proxy exec -- cargo build`
2. **Proxy checks policy** → Is this command allowed?
3. **Checkpoint created** → Automatic rollback point if configured
4. **Workspace mounted** → FUSE exposes the vault as a real directory
5. **Command executes** → Standard tools run normally, with configurable timeouts
6. **Result returned** → stdout, stderr, exit code, and changed files list

This is a **top-level command boundary**—cheap, practical, and designed for agent orchestration rather than syscall-level tracing.

## Production Features

- **Atomic transactions** — SQLite backend uses `BEGIN IMMEDIATE` for all filesystem mutations, eliminating race conditions during concurrent access
- **Execution timeouts** — Proxy commands support millisecond-level timeouts with automatic SIGKILL escalation
- **Mount session state machine** — Explicit mount/unmount lifecycle with double-unmount protection and automatic cleanup on drop
- **Backend deduplication** — Weak-reference cache ensures only one backend handle per vault, preventing resource leaks
- **Open file state tracking** — FUSE file handles track `Open → Dirty → Persisting → Flushed` states to eliminate persist races

## Commands

| Category | Commands |
|----------|----------|
| **Files** | `ls`, `cat`, `write`, `cp`, `mv`, `rm`, `tree`, `mkdir` |
| **Search** | `grep`, `find`, `search` |
| **Versioning** | `log`, `checkout`, `revert`, `diff` |
| **Vaults** | `vault create`, `vault list`, `vault use`, `vault fork`, `vault delete` |
| **Rollback** | `checkpoint save`, `checkpoint restore`, `checkpoint list` |
| **Runtime** | `mount`, `unmount`, `proxy exec` |
| **Maintenance** | `stats`, `audit`, `quota`, `gc`, `prune` |

Run `avfs --help` for the full command reference, or see the [command documentation](https://docs.neullabs.com/agentvfs/reference/commands).

## Documentation

Full documentation is available at **[docs.neullabs.com/agentvfs](https://docs.neullabs.com/agentvfs)**

- [Getting Started](https://docs.neullabs.com/agentvfs/getting-started) — Installation and first steps
- [Core Concepts](https://docs.neullabs.com/agentvfs/concepts) — Vaults, forks, checkpoints, mounts
- [Agent Integration](https://docs.neullabs.com/agentvfs/agent-integration) — Building agents with agentvfs
- [Proxy Boundary](https://docs.neullabs.com/agentvfs/proxy-boundary) — Execution model and policy
- [Command Reference](https://docs.neullabs.com/agentvfs/reference/commands) — All CLI commands

## License

MIT
