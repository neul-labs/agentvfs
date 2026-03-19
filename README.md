# agentvfs - Virtual Filesystem CLI

A command-line tool that implements a fully-featured virtual filesystem backed by embedded databases. Manage files, directories, and content using familiar shell commands without touching the real filesystem.

## Features

- **Familiar Commands**: Use `ls`, `cp`, `mv`, `rm`, `cat`, `mkdir`, and more
- **Pluggable Storage**: SQLite (default), Sled, LMDB, or RocksDB backends
- **Multiple Vaults**: Create and switch between independent databases
- **Version History**: Automatic versioning on every file change with rollback support
- **Metadata & Tags**: Add custom tags and metadata to any file
- **Full-Text Search**: Fast content search (FTS5 for SQLite, Tantivy for others)
- **Grep Support**: Regex-based content searching across files
- **External Commands**: Run bash commands on virtual files via exec or pipes
- **Interactive Shell**: REPL mode where you don't need to prefix commands with `avfs`
- **Agent-Friendly**: JSON output, snapshots, quotas, and audit logs for AI agent integration

## Installation

```bash
# From crates.io (once published)
cargo install agentvfs

# From source
git clone https://github.com/yourusername/agentvfs
cd agentvfs
cargo build --release
```

## Quick Start

```bash
# Create a new vault
avfs vault create myproject

# Create directories and files
avfs mkdir /docs
avfs write /docs/readme.txt "Hello, World!"

# List files
avfs ls /docs

# Read file contents
avfs cat /docs/readme.txt

# Copy and move files
avfs cp /docs/readme.txt /docs/backup.txt
avfs mv /docs/backup.txt /archive/

# Search for content
avfs grep "Hello" /docs/

# View version history
avfs log /docs/readme.txt

# Enter interactive shell (no avfs prefix needed)
avfs shell
```

## Command Reference

| Category | Commands |
|----------|----------|
| Navigation | `ls`, `cd`, `pwd`, `tree` |
| File Operations | `cp`, `mv`, `rm`, `cat`, `touch`, `write` |
| Directory Operations | `mkdir`, `rmdir` |
| Comparison | `diff` |
| Search | `grep`, `find` |
| Vault Management | `vault create`, `vault list`, `vault use`, `vault delete` |
| Import/Export | `import`, `export` |
| Versioning | `log`, `checkout`, `revert` |
| Metadata | `tag`, `untag`, `meta` |
| External Commands | `exec`, `pipe` |
| Maintenance | `prune`, `compact`, `gc` |
| Snapshots | `snapshot save`, `snapshot restore`, `snapshot list` |
| Audit | `audit`, `audit clear` |
| Shell | `shell`, `aliases` |

## Interactive Shell

Launch an interactive session where commands work without the `avfs` prefix:

```bash
$ avfs shell
[vault:myproject] / > ls
docs/
archive/
[vault:myproject] / > cd docs
[vault:myproject] /docs > cat readme.txt
Hello, World!
[vault:myproject] /docs > exit
```

## Storage Backends

avfs supports multiple embedded database backends:

| Backend | Best For | Notes |
|---------|----------|-------|
| **SQLite** (default) | General use | Built-in FTS5 search, great tooling |
| **Sled** | High write throughput | Pure Rust, modern architecture |
| **LMDB** | Read-heavy workloads | Memory-mapped, very fast reads |
| **RocksDB** | Large datasets | LSM-tree, good for SSDs |

```bash
# Create vault with specific backend
avfs vault create myproject --backend sqlite
avfs vault create logs --backend sled

# Migrate between backends
avfs vault migrate myproject --to lmdb
```

See [Storage Backends](docs/storage-backends.md) for details.

## AI Agent Integration

avfs is designed to work as a sandboxed filesystem for AI agents:

```python
import subprocess, json

def avfs(*args):
    result = subprocess.run(["avfs", "--json"] + list(args), capture_output=True, text=True)
    return json.loads(result.stdout)

# Save state before experiment
avfs("snapshot", "save", "checkpoint")

# Agent does work
avfs("mkdir", "/workspace")
avfs("write", "/workspace/code.py", "print('hello')")

# Rollback if needed
avfs("snapshot", "restore", "checkpoint")
```

**Key features for agents:**
- `--json` flag on all commands for structured output
- Snapshots for save/restore state
- Quotas to prevent runaway resource usage
- Audit logs for debugging

See [Agent Integration](docs/agent-integration.md) for full documentation.

## Agent Setup Script

For AI agents (Claude, Codex, etc.), use the setup script to create a workspace:

```bash
# Create and mount a vault, then cd into it
source scripts/agent-setup.sh my-workspace

# Or add this function to your .bashrc:
avfs-workspace() {
    local name="${1:-agent-workspace}"
    local mount="/tmp/avfs-$name"
    avfs vault create "$name" 2>/dev/null
    mkdir -p "$mount"
    avfs mount "$name" "$mount"
    cd "$mount"
}
```

## Documentation

- [Architecture Overview](docs/architecture.md)
- [Storage Backends](docs/storage-backends.md)
- [Data Model & Schema](docs/schema.md)
- [Command Reference](docs/commands.md)
- [External Commands](docs/exec.md)
- [Vault Management](docs/vaults.md)
- [Versioning](docs/versioning.md)
- [Maintenance & Pruning](docs/maintenance.md)
- [Metadata & Tags](docs/metadata.md)
- [Search & Grep](docs/search.md)
- [Interactive Shell](docs/shell.md)
- [Agent Integration](docs/agent-integration.md)

## Why agentvfs?

- **Isolation**: Keep project files separate from your real filesystem
- **Portability**: A single database file contains your entire filesystem
- **Flexibility**: Choose the storage backend that fits your workload
- **Version Control**: Built-in history without needing git for simple files
- **Searchability**: Fast full-text search across all content
- **Experimentation**: Test file operations without risk to real files

## Roadmap

See [ROADMAP.md](ROADMAP.md) for implementation status and planned features.

## License

MIT
