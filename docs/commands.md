# Command Reference

Complete reference for all vfs commands. All commands can be run with `vfs <command>` or directly in the interactive shell.

## Navigation Commands

### ls - List directory contents

```bash
vfs ls [OPTIONS] [PATH]
```

**Options:**
- `-l, --long` - Show detailed information (size, date, type)
- `-a, --all` - Include hidden files (starting with `.`)
- `-R, --recursive` - List subdirectories recursively
- `-t, --time` - Sort by modification time
- `-S, --size` - Sort by size

**Examples:**
```bash
vfs ls                    # List current directory
vfs ls /docs              # List specific directory
vfs ls -l /docs           # Detailed listing
vfs ls -laR /             # Full recursive listing
```

### cd - Change directory

```bash
vfs cd [PATH]
```

**Examples:**
```bash
vfs cd /docs              # Change to /docs
vfs cd ..                 # Go up one level
vfs cd                    # Go to root (/)
```

### pwd - Print working directory

```bash
vfs pwd
```

### tree - Display directory tree

```bash
vfs tree [OPTIONS] [PATH]
```

**Options:**
- `-L, --level <N>` - Limit depth to N levels
- `-d, --dirs-only` - Show only directories
- `--size` - Show file sizes

**Examples:**
```bash
vfs tree                  # Tree from current directory
vfs tree -L 2 /           # Tree with max depth 2
vfs tree --size /docs     # Tree with file sizes
```

## File Operations

### cat - Display file contents

```bash
vfs cat [OPTIONS] <PATH>...
```

**Options:**
- `-n, --number` - Number output lines
- `-v, --version <N>` - Show specific version

**Examples:**
```bash
vfs cat /docs/readme.txt          # Show file contents
vfs cat -n /src/main.rs           # With line numbers
vfs cat -v 3 /docs/readme.txt     # Show version 3
vfs cat file1.txt file2.txt       # Concatenate multiple files
```

### write - Write content to a file

```bash
vfs write [OPTIONS] <PATH> [CONTENT]
```

**Options:**
- `-a, --append` - Append instead of overwrite
- `--stdin` - Read content from stdin

**Examples:**
```bash
vfs write /docs/new.txt "Hello, World!"
vfs write -a /docs/log.txt "New line"
echo "piped content" | vfs write --stdin /docs/file.txt
```

### touch - Create empty file or update timestamp

```bash
vfs touch <PATH>...
```

**Examples:**
```bash
vfs touch /docs/newfile.txt
vfs touch file1.txt file2.txt file3.txt
```

### cp - Copy files or directories

```bash
vfs cp [OPTIONS] <SOURCE>... <DEST>
```

**Options:**
- `-r, --recursive` - Copy directories recursively
- `-n, --no-clobber` - Don't overwrite existing files
- `-v, --verbose` - Show files as they're copied

**Examples:**
```bash
vfs cp /docs/file.txt /backup/
vfs cp -r /docs /backup/docs
vfs cp file1.txt file2.txt /dest/
```

### mv - Move or rename files

```bash
vfs mv [OPTIONS] <SOURCE>... <DEST>
```

**Options:**
- `-n, --no-clobber` - Don't overwrite existing files
- `-v, --verbose` - Show files as they're moved

**Examples:**
```bash
vfs mv /docs/old.txt /docs/new.txt    # Rename
vfs mv /docs/file.txt /archive/        # Move
vfs mv file1.txt file2.txt /dest/      # Move multiple
```

### rm - Remove files or directories

```bash
vfs rm [OPTIONS] <PATH>...
```

**Options:**
- `-r, --recursive` - Remove directories recursively
- `-f, --force` - Don't prompt for confirmation
- `-v, --verbose` - Show files as they're removed

**Examples:**
```bash
vfs rm /docs/old.txt
vfs rm -r /temp
vfs rm -rf /cache/*
```

## Directory Operations

### mkdir - Create directories

```bash
vfs mkdir [OPTIONS] <PATH>...
```

**Options:**
- `-p, --parents` - Create parent directories as needed

**Examples:**
```bash
vfs mkdir /docs
vfs mkdir -p /deep/nested/directory
vfs mkdir dir1 dir2 dir3
```

### rmdir - Remove empty directories

```bash
vfs rmdir <PATH>...
```

**Examples:**
```bash
vfs rmdir /empty-dir
vfs rmdir dir1 dir2
```

## Comparison

### diff - Compare files

```bash
vfs diff [OPTIONS] <FILE1> <FILE2>
```

**Options:**
- `-u, --unified` - Unified diff format (default)
- `-c, --context` - Context diff format
- `--color` - Colorize output
- `-v, --version <N>` - Compare with version N

**Examples:**
```bash
vfs diff /docs/v1.txt /docs/v2.txt
vfs diff -v 1 /docs/readme.txt          # Current vs version 1
vfs diff --color file1.txt file2.txt
```

## Search Commands

### grep - Search file contents

```bash
vfs grep [OPTIONS] <PATTERN> [PATH]...
```

**Options:**
- `-i, --ignore-case` - Case-insensitive search
- `-r, --recursive` - Search recursively
- `-l, --files-with-matches` - Show only matching filenames
- `-n, --line-number` - Show line numbers
- `-c, --count` - Show only match count
- `-v, --invert-match` - Show non-matching lines
- `-A, --after <N>` - Show N lines after match
- `-B, --before <N>` - Show N lines before match
- `-C, --context <N>` - Show N lines around match

**Examples:**
```bash
vfs grep "TODO" /src/                 # Search in directory
vfs grep -rn "function" /             # Recursive with line numbers
vfs grep -i "error" /logs/*.log       # Case-insensitive with glob
vfs grep -l "import" /src/**/*.rs     # Files containing pattern
vfs grep -C 3 "bug" /src/main.rs      # With context
```

### search - Full-text search

```bash
vfs search [OPTIONS] <QUERY> [PATH]
```

Full-text search using FTS5 (SQLite) or Tantivy (other backends).

**Options:**
- `-i, --ignore-case` - Case-insensitive search
- `-w, --word` - Match whole words only
- `-l, --files-only` - Show only filenames
- `-c, --count` - Show match counts
- `--limit <N>` - Limit results
- `--rebuild` - Rebuild search index
- `--status` - Show index status

**Examples:**
```bash
vfs search "database connection"      # Search all files
vfs search "TODO" /src/               # Search in directory
vfs search -l "error"                 # Just filenames
vfs search "config NOT test"          # Boolean query
vfs search "data*"                    # Prefix matching
```

### find - Find files by name or attributes

```bash
vfs find [PATH] [OPTIONS]
```

**Options:**
- `-name <PATTERN>` - Match filename pattern (glob)
- `-type <TYPE>` - Filter by type (`f` for file, `d` for directory)
- `-size <SIZE>` - Filter by size (`+1M`, `-100K`)
- `-mtime <DAYS>` - Modified within N days
- `-tag <TAG>` - Filter by tag
- `-exec <CMD>` - Execute command on matches

**Examples:**
```bash
vfs find / -name "*.txt"
vfs find /src -type f -name "*.rs"
vfs find / -size +1M
vfs find / -mtime -7                  # Modified in last 7 days
vfs find / -tag important
vfs find / -name "*.log" -exec rm {}
```

## Vault Management

### vault create - Create a new vault

```bash
vfs vault create <NAME> [OPTIONS]
```

**Options:**
- `--path <PATH>` - Custom database path

**Examples:**
```bash
vfs vault create myproject
vfs vault create backup --path /mnt/external/backup.vfs
vfs vault create fast --backend sled
```

### vault list - List all vaults

```bash
vfs vault list [OPTIONS]
```

**Options:**
- `--json` - JSON output

### vault use - Switch to a vault

```bash
vfs vault use <NAME>
```

### vault delete - Delete a vault

```bash
vfs vault delete <NAME> [OPTIONS]
```

**Options:**
- `--force` - Delete without confirmation

### vault info - Show vault information

```bash
vfs vault info [NAME] [OPTIONS]
```

**Options:**
- `--json` - JSON output

Shows size, file count, version count, limits, and settings.

### vault config - Configure vault settings

```bash
vfs vault config [KEY] [VALUE]
```

View or modify vault settings.

**Examples:**
```bash
vfs vault config                      # Show all settings
vfs vault config max_size_mb 100      # Set max vault size
vfs vault config max_files 10000      # Set max file count
vfs vault config max_file_size_mb 10  # Set max single file size
vfs vault config prune_strategy keep_n
vfs vault config prune_keep_count 10
```

### vault import - Register external vault

```bash
vfs vault import <NAME> <PATH>
```

Register an existing vault database file.

**Examples:**
```bash
vfs vault import shared ~/Downloads/shared.vfs
```

### vault stats - Show detailed statistics

```bash
vfs vault stats [OPTIONS]
```

**Options:**
- `--json` - JSON output
- `--backend-info` - Include backend-specific info
- `--versions` - Show version distribution

**Examples:**
```bash
vfs vault stats                       # Storage breakdown
vfs vault stats --versions            # Files by version count
```

## Import/Export

### import - Import from real filesystem

```bash
vfs import [OPTIONS] <REAL_PATH> <VIRTUAL_PATH>
```

**Options:**
- `-r, --recursive` - Import directories recursively
- `-v, --verbose` - Show files as imported

**Examples:**
```bash
vfs import ~/documents/report.pdf /docs/
vfs import -r ~/project/src /src
```

### export - Export to real filesystem

```bash
vfs export [OPTIONS] <VIRTUAL_PATH> <REAL_PATH>
```

**Options:**
- `-r, --recursive` - Export directories recursively
- `-v, --verbose` - Show files as exported
- `--version <N>` - Export specific version

**Examples:**
```bash
vfs export /docs/report.pdf ~/Downloads/
vfs export -r /src ~/backup/src
vfs export --version 5 /docs/file.txt ~/old-version.txt
```

## Versioning Commands

### log - Show version history

```bash
vfs log [OPTIONS] <PATH>
```

**Options:**
- `-n, --number <N>` - Show only last N versions
- `--oneline` - Compact format

**Examples:**
```bash
vfs log /docs/readme.txt
vfs log -n 5 /src/main.rs
vfs log --oneline /docs/
```

### checkout - Restore a previous version

```bash
vfs checkout <PATH> <VERSION>
```

**Examples:**
```bash
vfs checkout /docs/readme.txt 3       # Restore version 3
```

### revert - Revert to previous version

```bash
vfs revert <PATH>
```

Reverts to the immediately previous version (creates a new version).

## Metadata Commands

### tag - Add tags to files

```bash
vfs tag [OPTIONS] <PATH> <TAG>...
```

**Options:**
- `-r, --recursive` - Tag files recursively
- `--create <NAME>` - Create a new tag (with optional `--color`)
- `--delete <NAME>` - Delete a tag
- `--rename <OLD> <NEW>` - Rename a tag
- `--list` - List all tags in vault
- `--copy <SRC> <DST>` - Copy tags from one file to another
- `--color <HEX>` - Set tag color (with `--create`)

**Examples:**
```bash
vfs tag /docs/report.pdf important urgent
vfs tag /src/*.rs code rust
vfs tag -r /project/ work             # Recursive
vfs tag --create important --color red
vfs tag --list                        # List all tags
vfs tag --copy /template.txt /new.txt
```

### untag - Remove tags from files

```bash
vfs untag <PATH> <TAG>...
```

### meta - View or set metadata

```bash
vfs meta [OPTIONS] <PATH> [KEY] [VALUE]
```

**Options:**
- `--unset <KEY>` - Remove a metadata key
- `--export <PATH>` - Export metadata to JSON file
- `--import <FILE>` - Import metadata from JSON file
- `--copy <SRC> <DST>` - Copy metadata between files

**Examples:**
```bash
vfs meta /docs/file.txt                     # Show all metadata
vfs meta /docs/file.txt author              # Get specific key
vfs meta /docs/file.txt author "John Doe"   # Set value
vfs meta --unset /docs/file.txt status      # Remove key
vfs meta --export /docs/ > metadata.json    # Export
vfs meta --import metadata.json             # Import
```

## External Commands

### exec - Run external command on file

```bash
vfs exec [OPTIONS] '<COMMAND>' <PATH>...
```

Extracts file to temp, runs command, re-imports result. Supports glob patterns.

**Options:**
- `-n, --no-reimport` - Run command but don't re-import result
- `-v, --verbose` - Show temp file location and command
- `--dry-run` - Show what would be executed
- `-e, --env <KEY=VALUE>` - Set environment variable
- `--timeout <SECONDS>` - Kill command after timeout (default: 60)
- `--parallel <N>` - Process N files concurrently
- `--fail-fast` - Stop on first error (with globs)
- `--shell` - Use shell interpretation (less safe)
- `--allow-network` - Allow network access

**Examples:**
```bash
vfs exec 'sed -i s/foo/bar/g' /docs/file.txt
vfs exec 'sort' /data/names.txt
vfs exec 'jq .' /config/settings.json
vfs exec 'gzip' '/logs/*.log'         # Glob pattern
vfs exec --parallel 4 'process' '/data/*.csv'
```

### pipe - Pipe operations (via shell)

```bash
vfs cat <PATH> | <command> | vfs write <PATH>
```

**Examples:**
```bash
vfs cat /data.txt | sort | uniq | vfs write /sorted.txt
vfs cat /log.txt | grep ERROR | vfs write /errors.txt
```

## Maintenance Commands

### prune - Remove old versions

```bash
vfs prune [OPTIONS]
```

**Options:**
- `--keep <N>` - Keep last N versions per file
- `--older-than <DAYS>` - Remove versions older than N days
- `--dry-run` - Show what would be removed

### compact - Reclaim space

```bash
vfs compact
```

Removes unreferenced content and optimizes storage (VACUUM for SQLite, compaction for others).

### gc - Garbage collection

```bash
vfs gc [OPTIONS]
```

**Options:**
- `--dry-run` - Show what would be removed
- `--stats` - Show garbage collection statistics

### maintain - Full maintenance routine

```bash
vfs maintain [OPTIONS]
```

Runs prune, gc, and compact in sequence.

**Options:**
- `--dry-run` - Show what would be done

## Shell Commands

### shell - Start interactive shell

```bash
vfs shell
```

Launches REPL where commands work without `vfs` prefix.

### aliases - Generate shell aliases

```bash
vfs aliases [OPTIONS]
```

**Options:**
- `--format <FMT>` - Shell format (`bash`, `zsh`, `fish`)
- `--prefix <PREFIX>` - Command prefix (default: none)

**Examples:**
```bash
eval "$(vfs aliases)"                 # Activate aliases
eval "$(vfs aliases --prefix vfs-)"   # Use vfs-ls, vfs-cp, etc.
```

## Snapshot Commands

### snapshot save - Create a snapshot

```bash
vfs snapshot save [NAME]
```

Saves the current vault state. If no name is given, an auto-generated name is used.

**Examples:**
```bash
vfs snapshot save before-experiment
vfs snapshot save                      # Auto-generated name
```

### snapshot list - List snapshots

```bash
vfs snapshot list [OPTIONS]
```

**Options:**
- `--json` - JSON output

### snapshot restore - Restore a snapshot

```bash
vfs snapshot restore <NAME>
```

Restores vault to a previous snapshot. Current state is auto-saved before restore.

**Examples:**
```bash
vfs snapshot restore before-experiment
```

### snapshot delete - Delete a snapshot

```bash
vfs snapshot delete <NAME>
```

## Audit Commands

### audit - View operation history

```bash
vfs audit [OPTIONS]
```

**Options:**
- `--json` - JSON output
- `--limit <N>` - Show last N entries (default: 50)
- `--op <OP>` - Filter by operation (can repeat)
- `--path <PATH>` - Filter by path prefix
- `--since <TIMESTAMP>` - Filter by time

**Examples:**
```bash
vfs audit                              # Recent operations
vfs audit --json --limit 100           # JSON output
vfs audit --op write --op rm           # Only write/rm operations
vfs audit --path /docs/                # Operations in /docs
```

### audit clear - Clear audit log

```bash
vfs audit clear
```

## Global Options

These options work with all commands:

- `--vault <NAME>` - Use specific vault
- `--json` - Output in JSON format (for programmatic use)
- `--help` - Show help for command
- `--version` - Show vfs version
- `--quiet` - Suppress non-error output
- `--verbose` - Show detailed output

## JSON Output

When `--json` is specified, all commands output structured JSON:

```bash
# Success
$ vfs ls --json /docs
{
  "path": "/docs",
  "entries": [...]
}

# Error
$ vfs cat --json /nonexistent
{
  "error": "NotFound",
  "message": "File not found: /nonexistent"
}
```

Exit codes: `0` for success, `1` for errors. Always check both exit code and JSON.
