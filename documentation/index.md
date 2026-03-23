# agentvfs

**Sandboxed filesystem for AI agents**

A database-backed virtual filesystem designed for AI agents. Create isolated workspaces, track file versions, search content, and roll back changes - all without touching the real filesystem.

## What's New

- :material-speedometer: **High-performance allocator** - mimalloc for faster memory operations
- :material-cached: **Zero-copy caching** - rkyv-based cache for frequently accessed data
- :material-desktop-classic: **Cross-platform** - Pre-built binaries for Linux, macOS, and Windows
- :material-console-line: **Interactive shell** - REPL mode with tab completion and history

## Features

<div class="grid cards" markdown>

- :material-robot: **Built for AI Agents**
  JSON output, snapshots, quotas, and audit logs for seamless integration

- :material-folder-multiple: **Familiar Commands**
  Use `ls`, `cp`, `mv`, `rm`, `cat`, `mkdir`, and more

- :material-history: **Version History**
  Every change tracked, instant rollback to any version

- :material-magnify: **Full-Text Search**
  Fast content search with FTS5, grep with regex support

- :material-safe-square: **Multiple Vaults**
  Isolated workspaces, each in its own database

- :material-harddisk: **FUSE Mount**
  Mount vaults as real directories (Linux/macOS)

</div>

## Quick Example

```bash
# Create an isolated workspace
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

# Interactive shell (no prefix needed)
avfs shell
```

## For AI Agents

```python
import subprocess
import json

def avfs(*args):
    result = subprocess.run(
        ["avfs", "--json"] + list(args),
        capture_output=True, text=True
    )
    return json.loads(result.stdout) if result.stdout else None

# Checkpoint before risky operations
avfs("snapshot", "save", "before-changes")

# Work with files
avfs("mkdir", "/workspace")
avfs("write", "/workspace/code.py", "# Generated")

# Roll back if needed
avfs("snapshot", "restore", "before-changes")
```

## Why agentvfs?

| Feature | Benefit |
|---------|---------|
| **Isolation** | Sandboxed filesystem - no risk to real files |
| **Versioning** | Every change tracked, instant rollback |
| **Searchable** | Full-text search across all content |
| **Portable** | Single database file, easy to backup |
| **Fast** | SQLite backend, mimalloc allocator, rkyv caching |

## Getting Started

- [Installation](getting-started/installation.md) - Install avfs on your system
- [Quick Start](getting-started/quickstart.md) - Learn the basics in 5 minutes
- [Core Concepts](getting-started/concepts.md) - Understand vaults, versioning, and more

## Command Categories

| Category | Commands |
|----------|----------|
| **Files** | `ls`, `cat`, `write`, `cp`, `mv`, `rm`, `tree` |
| **Directories** | `mkdir`, `pwd` |
| **Search** | `grep`, `find`, `search` |
| **Versioning** | `log`, `checkout`, `revert`, `diff` |
| **Vaults** | `vault create`, `vault list`, `vault use`, `vault delete` |
| **Snapshots** | `snapshot save`, `snapshot restore`, `snapshot list` |
| **Maintenance** | `stats`, `prune`, `gc`, `compact` |
| **Shell** | `shell`, `aliases` |
| **FUSE** | `mount`, `unmount` |

See the [Command Reference](reference/commands.md) for complete documentation.

## License

MIT
