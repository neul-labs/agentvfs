# VFS - Virtual Filesystem CLI

A command-line tool that implements a fully-featured virtual filesystem backed by embedded databases. Manage files, directories, and content using familiar shell commands without touching the real filesystem.

## Features

<div class="grid cards" markdown>

- :material-folder-multiple: **Familiar Commands**
  Use `ls`, `cp`, `mv`, `rm`, `cat`, `mkdir`, and more

- :material-database: **Pluggable Storage**
  SQLite (default), Sled, LMDB, or RocksDB backends

- :material-safe-square: **Multiple Vaults**
  Create and switch between independent databases

- :material-history: **Version History**
  Automatic versioning on every file change with rollback support

- :material-tag-multiple: **Metadata & Tags**
  Add custom tags and metadata to any file

- :material-magnify: **Full-Text Search**
  Fast content search (FTS5 for SQLite, Tantivy for others)

- :material-regex: **Grep Support**
  Regex-based content searching across files

- :material-console: **External Commands**
  Run bash commands on virtual files via exec or pipes

- :material-console-line: **Interactive Shell**
  REPL mode where you don't need to prefix commands with `avfs`

- :material-robot: **Agent-Friendly**
  JSON output, snapshots, quotas, and audit logs for AI agent integration

- :material-harddisk: **FUSE Mount**
  Mount vaults as real directories (Linux/macOS)

</div>

## Quick Example

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

# Search for content
avfs grep "Hello" /docs/

# View version history
avfs log /docs/readme.txt

# Enter interactive shell
avfs shell
```

## Why VFS?

| Use Case | Benefit |
|----------|---------|
| **Isolation** | Keep project files separate from your real filesystem |
| **Portability** | A single database file contains your entire filesystem |
| **Flexibility** | Choose the storage backend that fits your workload |
| **Version Control** | Built-in history without needing git for simple files |
| **Searchability** | Fast full-text search across all content |
| **Experimentation** | Test file operations without risk to real files |

## Getting Started

Ready to dive in? Start with these guides:

- [Installation](getting-started/installation.md) - Get VFS installed on your system
- [Quick Start](getting-started/quickstart.md) - Learn the basics in 5 minutes
- [Core Concepts](getting-started/concepts.md) - Understand vaults, versioning, and more

## Command Categories

| Category | Commands |
|----------|----------|
| **Navigation** | `ls`, `pwd`, `tree` |
| **File Operations** | `cp`, `mv`, `rm`, `cat`, `write` |
| **Directory Operations** | `mkdir` |
| **Versioning** | `log`, `checkout`, `revert`, `diff` |
| **Search** | `search`, `grep`, `find` |
| **Vault Management** | `vault create`, `vault list`, `vault use`, `vault delete` |
| **Import/Export** | `import`, `export`, `exec` |
| **Metadata** | `tag`, `untag`, `meta` |
| **Maintenance** | `prune`, `compact`, `gc`, `maintain` |
| **Snapshots** | `snapshot save`, `snapshot restore`, `snapshot list` |
| **Shell** | `shell`, `aliases` |
| **FUSE** | `mount`, `unmount` |

See the [Command Reference](reference/commands.md) for complete documentation.

## License

MIT
