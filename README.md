# agentvfs

**Sandboxed filesystem for AI agents**

[![Crates.io](https://img.shields.io/crates/v/agentvfs.svg)](https://crates.io/crates/agentvfs)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Downloads](https://img.shields.io/crates/d/agentvfs.svg)](https://crates.io/crates/agentvfs)

A database-backed virtual filesystem designed for AI agents. Create isolated workspaces, track file versions, search content, and roll back changes - all without touching the real filesystem.

## What's New

- **High-performance allocator** - mimalloc for faster memory operations
- **Zero-copy caching** - rkyv-based cache for frequently accessed data
- **Cross-platform** - Pre-built binaries for Linux, macOS, and Windows
- **Interactive shell** - REPL mode with tab completion and history

## Installation

### Quick Install

```bash
curl -sSfL https://raw.githubusercontent.com/neul-labs/agentvfs/main/scripts/install.sh | bash
```

### From crates.io

```bash
cargo install agentvfs
```

### From Source

```bash
git clone https://github.com/neul-labs/agentvfs
cd agentvfs
cargo build --release
```

### Pre-built Binaries

Download from [GitHub Releases](https://github.com/neul-labs/agentvfs/releases):

| Platform | Download |
|----------|----------|
| Linux x86_64 | `avfs-VERSION-linux-x86_64.tar.gz` |
| Linux ARM64 | `avfs-VERSION-linux-aarch64.tar.gz` |
| macOS x86_64 | `avfs-VERSION-darwin-x86_64.tar.gz` |
| macOS ARM64 | `avfs-VERSION-darwin-aarch64.tar.gz` |
| Windows x86_64 | `avfs-VERSION-windows-x86_64.zip` |

## Quick Start

```bash
# Create a vault (isolated workspace)
avfs vault create myproject

# Work with files
avfs mkdir /src
avfs write /src/main.py "print('hello')"
avfs cat /src/main.py

# Search and navigate
avfs grep "hello" /
avfs tree /

# Version control
avfs log /src/main.py
avfs checkout /src/main.py --version 1

# Interactive shell
avfs shell
```

## Interactive Shell

Launch a REPL where commands work without the `avfs` prefix:

```
$ avfs shell
avfs interactive shell
Type 'help' for available commands, 'exit' to quit.

myproject> mkdir /docs
myproject> write /docs/notes.txt "Meeting notes..."
myproject> ls /docs
notes.txt
myproject> cat /docs/notes.txt
Meeting notes...
myproject> exit
Goodbye!
```

**Shell features:**
- Tab completion for commands and paths
- Command history (persisted in `~/.avfs/history`)
- All standard commands available
- Ctrl+C to cancel, Ctrl+D to exit

## Commands

| Category | Commands |
|----------|----------|
| **Files** | `ls`, `cat`, `write`, `cp`, `mv`, `rm`, `tree` |
| **Directories** | `mkdir`, `pwd` |
| **Search** | `grep`, `find`, `search` |
| **Versioning** | `log`, `checkout`, `revert`, `diff` |
| **Metadata** | `tag`, `untag`, `meta` |
| **Import/Export** | `import`, `export`, `exec` |
| **Vaults** | `vault create`, `vault list`, `vault use`, `vault delete` |
| **Maintenance** | `stats`, `prune`, `gc`, `compact` |
| **Snapshots** | `snapshot save`, `snapshot restore`, `snapshot list` |

## For AI Agents

agentvfs is designed for AI agent workflows:

```python
import subprocess
import json

def avfs(*args):
    result = subprocess.run(
        ["avfs", "--json"] + list(args),
        capture_output=True, text=True
    )
    return json.loads(result.stdout) if result.stdout else None

# Create isolated workspace
avfs("vault", "create", "agent-workspace")

# Save checkpoint before risky operations
avfs("snapshot", "save", "before-changes")

# Work with files
avfs("mkdir", "/workspace")
avfs("write", "/workspace/code.py", "# Generated code")

# Roll back if needed
avfs("snapshot", "restore", "before-changes")
```

**Agent-friendly features:**
- `--json` flag for structured output on all commands
- Snapshots for save/restore state
- Quotas to prevent runaway resource usage
- Audit logs for debugging
- FUSE mount for native filesystem access

## Why agentvfs?

| Feature | Benefit |
|---------|---------|
| **Isolation** | Sandboxed filesystem - no risk to real files |
| **Versioning** | Every change tracked, instant rollback |
| **Searchable** | Full-text search across all content |
| **Portable** | Single database file, easy to backup |
| **Fast** | SQLite backend, mimalloc allocator, rkyv caching |

## Documentation

- [Quick Start Guide](documentation/getting-started/quickstart.md)
- [Shell Usage](documentation/user-guide/shell.md)
- [Vault Management](documentation/user-guide/vaults.md)
- [Versioning](documentation/user-guide/versioning.md)
- [Agent Integration](documentation/advanced/agent-integration.md)
- [FUSE Mount](documentation/advanced/fuse-mount.md)
- [Command Reference](documentation/reference/commands.md)

## License

MIT
