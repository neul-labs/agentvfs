# Command Reference

Complete reference for all avfs commands. All commands can be run with `avfs <command>` or directly in the interactive shell.

## Navigation Commands

### ls - List directory contents

```bash
avfs ls [OPTIONS] [PATH]
```

**Options:**
- `-l, --long` - Show detailed information (size, date, type)
- `-a, --all` - Include hidden files (starting with `.`)
- `-R, --recursive` - List subdirectories recursively
- `-t, --time` - Sort by modification time
- `-S, --size` - Sort by size

**Examples:**
```bash
avfs ls                    # List current directory
avfs ls /docs              # List specific directory
avfs ls -l /docs           # Detailed listing
avfs ls -laR /             # Full recursive listing
```

### cd - Change directory

```bash
avfs cd [PATH]
```

**Examples:**
```bash
avfs cd /docs              # Change to /docs
avfs cd ..                 # Go up one level
avfs cd                    # Go to root (/)
```

### pwd - Print working directory

```bash
avfs pwd
```

### tree - Display directory tree

```bash
avfs tree [OPTIONS] [PATH]
```

**Options:**
- `-L, --level <N>` - Limit depth to N levels
- `-d, --dirs-only` - Show only directories
- `--size` - Show file sizes

**Examples:**
```bash
avfs tree                  # Tree from current directory
avfs tree -L 2 /           # Tree with max depth 2
avfs tree --size /docs     # Tree with file sizes
```

## File Operations

### cat - Display file contents

```bash
avfs cat [OPTIONS] <PATH>...
```

**Options:**
- `-n, --number` - Number output lines
- `-v, --version <N>` - Show specific version

**Examples:**
```bash
avfs cat /docs/readme.txt          # Show file contents
avfs cat -n /src/main.rs           # With line numbers
avfs cat -v 3 /docs/readme.txt     # Show version 3
avfs cat file1.txt file2.txt       # Concatenate multiple files
```

### write - Write content to a file

```bash
avfs write [OPTIONS] <PATH> [CONTENT]
```

**Options:**
- `-a, --append` - Append instead of overwrite
- `--stdin` - Read content from stdin

**Examples:**
```bash
avfs write /docs/new.txt "Hello, World!"
avfs write -a /docs/log.txt "New line"
echo "piped content" | avfs write --stdin /docs/file.txt
```

### touch - Create empty file or update timestamp

```bash
avfs touch <PATH>...
```

**Examples:**
```bash
avfs touch /docs/newfile.txt
avfs touch file1.txt file2.txt file3.txt
```

### cp - Copy files or directories

```bash
avfs cp [OPTIONS] <SOURCE>... <DEST>
```

**Options:**
- `-r, --recursive` - Copy directories recursively
- `-n, --no-clobber` - Don't overwrite existing files
- `-v, --verbose` - Show files as they're copied

**Examples:**
```bash
avfs cp /docs/file.txt /backup/
avfs cp -r /docs /backup/docs
avfs cp file1.txt file2.txt /dest/
```

### mv - Move or rename files

```bash
avfs mv [OPTIONS] <SOURCE>... <DEST>
```

**Options:**
- `-n, --no-clobber` - Don't overwrite existing files
- `-v, --verbose` - Show files as they're moved

**Examples:**
```bash
avfs mv /docs/old.txt /docs/new.txt    # Rename
avfs mv /docs/file.txt /archive/        # Move
avfs mv file1.txt file2.txt /dest/      # Move multiple
```

### rm - Remove files or directories

```bash
avfs rm [OPTIONS] <PATH>...
```

**Options:**
- `-r, --recursive` - Remove directories recursively
- `-f, --force` - Don't prompt for confirmation
- `-v, --verbose` - Show files as they're removed

**Examples:**
```bash
avfs rm /docs/old.txt
avfs rm -r /temp
avfs rm -rf /cache/*
```

## Directory Operations

### mkdir - Create directories

```bash
avfs mkdir [OPTIONS] <PATH>...
```

**Options:**
- `-p, --parents` - Create parent directories as needed

**Examples:**
```bash
avfs mkdir /docs
avfs mkdir -p /deep/nested/directory
avfs mkdir dir1 dir2 dir3
```

### rmdir - Remove empty directories

```bash
avfs rmdir <PATH>...
```

**Examples:**
```bash
avfs rmdir /empty-dir
avfs rmdir dir1 dir2
```

## Comparison

### diff - Compare files

```bash
avfs diff [OPTIONS] <FILE1> <FILE2>
```

**Options:**
- `-u, --unified` - Unified diff format (default)
- `-c, --context` - Context diff format
- `--color` - Colorize output
- `-v, --version <N>` - Compare with version N

**Examples:**
```bash
avfs diff /docs/v1.txt /docs/v2.txt
avfs diff -v 1 /docs/readme.txt          # Current vs version 1
avfs diff --color file1.txt file2.txt
```

## Search Commands

### grep - Search file contents

```bash
avfs grep [OPTIONS] <PATTERN> [PATH]...
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
avfs grep "TODO" /src/                 # Search in directory
avfs grep -rn "function" /             # Recursive with line numbers
avfs grep -i "error" /logs/*.log       # Case-insensitive with glob
avfs grep -l "import" /src/**/*.rs     # Files containing pattern
avfs grep -C 3 "bug" /src/main.rs      # With context
```

### search - Full-text search

```bash
avfs search [OPTIONS] <QUERY> [PATH]
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
avfs search "database connection"      # Search all files
avfs search "TODO" /src/               # Search in directory
avfs search -l "error"                 # Just filenames
avfs search "config NOT test"          # Boolean query
avfs search "data*"                    # Prefix matching
```

### find - Find files by name or attributes

```bash
avfs find [PATH] [OPTIONS]
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
avfs find / -name "*.txt"
avfs find /src -type f -name "*.rs"
avfs find / -size +1M
avfs find / -mtime -7                  # Modified in last 7 days
avfs find / -tag important
avfs find / -name "*.log" -exec rm {}
```

## Vault Management

### vault create - Create a new vault

```bash
avfs vault create <NAME> [OPTIONS]
```

**Options:**
- `--path <PATH>` - Custom database path

**Examples:**
```bash
avfs vault create myproject
avfs vault create backup --path /mnt/external/backup.avfs
avfs vault create fast --backend sled
```

### vault list - List all vaults

```bash
avfs vault list [OPTIONS]
```

**Options:**
- `--json` - JSON output

### vault use - Switch to a vault

```bash
avfs vault use <NAME>
```

### vault delete - Delete a vault

```bash
avfs vault delete <NAME> [OPTIONS]
```

**Options:**
- `--force` - Delete without confirmation

### vault info - Show vault information

```bash
avfs vault info [NAME] [OPTIONS]
```

**Options:**
- `--json` - JSON output

Shows size, file count, version count, limits, and settings.

### vault config - Configure vault settings

```bash
avfs vault config [KEY] [VALUE]
```

View or modify vault settings.

**Examples:**
```bash
avfs vault config                      # Show all settings
avfs vault config max_size_mb 100      # Set max vault size
avfs vault config max_files 10000      # Set max file count
avfs vault config max_file_size_mb 10  # Set max single file size
avfs vault config prune_strategy keep_n
avfs vault config prune_keep_count 10
```

### vault import - Register external vault

```bash
avfs vault import <NAME> <PATH>
```

Register an existing vault database file.

**Examples:**
```bash
avfs vault import shared ~/Downloads/shared.avfs
```

### vault stats - Show detailed statistics

```bash
avfs vault stats [OPTIONS]
```

**Options:**
- `--json` - JSON output
- `--backend-info` - Include backend-specific info
- `--versions` - Show version distribution

**Examples:**
```bash
avfs vault stats                       # Storage breakdown
avfs vault stats --versions            # Files by version count
```

## Import/Export

### import - Import from real filesystem

```bash
avfs import [OPTIONS] <REAL_PATH> <VIRTUAL_PATH>
```

**Options:**
- `-r, --recursive` - Import directories recursively
- `-v, --verbose` - Show files as imported

**Examples:**
```bash
avfs import ~/documents/report.pdf /docs/
avfs import -r ~/project/src /src
```

### export - Export to real filesystem

```bash
avfs export [OPTIONS] <VIRTUAL_PATH> <REAL_PATH>
```

**Options:**
- `-r, --recursive` - Export directories recursively
- `-v, --verbose` - Show files as exported
- `--version <N>` - Export specific version

**Examples:**
```bash
avfs export /docs/report.pdf ~/Downloads/
avfs export -r /src ~/backup/src
avfs export --version 5 /docs/file.txt ~/old-version.txt
```

## Versioning Commands

### log - Show version history

```bash
avfs log [OPTIONS] <PATH>
```

**Options:**
- `-n, --number <N>` - Show only last N versions
- `--oneline` - Compact format

**Examples:**
```bash
avfs log /docs/readme.txt
avfs log -n 5 /src/main.rs
avfs log --oneline /docs/
```

### checkout - Restore a previous version

```bash
avfs checkout <PATH> <VERSION>
```

**Examples:**
```bash
avfs checkout /docs/readme.txt 3       # Restore version 3
```

### revert - Revert to previous version

```bash
avfs revert <PATH>
```

Reverts to the immediately previous version (creates a new version).

## Metadata Commands

### tag - Add tags to files

```bash
avfs tag [OPTIONS] <PATH> <TAG>...
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
avfs tag /docs/report.pdf important urgent
avfs tag /src/*.rs code rust
avfs tag -r /project/ work             # Recursive
avfs tag --create important --color red
avfs tag --list                        # List all tags
avfs tag --copy /template.txt /new.txt
```

### untag - Remove tags from files

```bash
avfs untag <PATH> <TAG>...
```

### meta - View or set metadata

```bash
avfs meta [OPTIONS] <PATH> [KEY] [VALUE]
```

**Options:**
- `--unset <KEY>` - Remove a metadata key
- `--export <PATH>` - Export metadata to JSON file
- `--import <FILE>` - Import metadata from JSON file
- `--copy <SRC> <DST>` - Copy metadata between files

**Examples:**
```bash
avfs meta /docs/file.txt                     # Show all metadata
avfs meta /docs/file.txt author              # Get specific key
avfs meta /docs/file.txt author "John Doe"   # Set value
avfs meta --unset /docs/file.txt status      # Remove key
avfs meta --export /docs/ > metadata.json    # Export
avfs meta --import metadata.json             # Import
```

## External Commands

### exec - Run external command on file

```bash
avfs exec [OPTIONS] '<COMMAND>' <PATH>...
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
avfs exec 'sed -i s/foo/bar/g' /docs/file.txt
avfs exec 'sort' /data/names.txt
avfs exec 'jq .' /config/settings.json
avfs exec 'gzip' '/logs/*.log'         # Glob pattern
avfs exec --parallel 4 'process' '/data/*.csv'
```

### pipe - Pipe operations (via shell)

```bash
avfs cat <PATH> | <command> | avfs write <PATH>
```

**Examples:**
```bash
avfs cat /data.txt | sort | uniq | avfs write /sorted.txt
avfs cat /log.txt | grep ERROR | avfs write /errors.txt
```

## Maintenance Commands

### prune - Remove old versions

```bash
avfs prune [OPTIONS]
```

**Options:**
- `--keep <N>` - Keep last N versions per file
- `--older-than <DAYS>` - Remove versions older than N days
- `--dry-run` - Show what would be removed

### compact - Reclaim space

```bash
avfs compact
```

Removes unreferenced content and optimizes storage (VACUUM for SQLite, compaction for others).

### gc - Garbage collection

```bash
avfs gc [OPTIONS]
```

**Options:**
- `--dry-run` - Show what would be removed
- `--stats` - Show garbage collection statistics

### maintain - Full maintenance routine

```bash
avfs maintain [OPTIONS]
```

Runs prune, gc, and compact in sequence.

**Options:**
- `--dry-run` - Show what would be done

## Shell Commands

### shell - Start interactive shell

```bash
avfs shell
```

Launches REPL where commands work without `avfs` prefix.

### aliases - Generate shell aliases

```bash
avfs aliases [OPTIONS]
```

**Options:**
- `--format <FMT>` - Shell format (`bash`, `zsh`, `fish`)
- `--prefix <PREFIX>` - Command prefix (default: none)

**Examples:**
```bash
eval "$(avfs aliases)"                 # Activate aliases
eval "$(avfs aliases --prefix avfs-)"   # Use avfs-ls, avfs-cp, etc.
```

## Snapshot Commands

### snapshot save - Create a snapshot

```bash
avfs snapshot save [NAME]
```

Saves the current vault state. If no name is given, an auto-generated name is used.

**Examples:**
```bash
avfs snapshot save before-experiment
avfs snapshot save                      # Auto-generated name
```

### snapshot list - List snapshots

```bash
avfs snapshot list [OPTIONS]
```

**Options:**
- `--json` - JSON output

### snapshot restore - Restore a snapshot

```bash
avfs snapshot restore <NAME>
```

Restores vault to a previous snapshot. Current state is auto-saved before restore.

**Examples:**
```bash
avfs snapshot restore before-experiment
```

### snapshot delete - Delete a snapshot

```bash
avfs snapshot delete <NAME>
```

## Audit Commands

### audit - View operation history

```bash
avfs audit [OPTIONS]
```

**Options:**
- `--json` - JSON output
- `--limit <N>` - Show last N entries (default: 50)
- `--op <OP>` - Filter by operation (can repeat)
- `--path <PATH>` - Filter by path prefix
- `--since <TIMESTAMP>` - Filter by time

**Examples:**
```bash
avfs audit                              # Recent operations
avfs audit --json --limit 100           # JSON output
avfs audit --op write --op rm           # Only write/rm operations
avfs audit --path /docs/                # Operations in /docs
```

### audit clear - Clear audit log

```bash
avfs audit clear
```

## Global Options

These options work with all commands:

- `--vault <NAME>` - Use specific vault
- `--json` - Output in JSON format (for programmatic use)
- `--help` - Show help for command
- `--version` - Show avfs version
- `--quiet` - Suppress non-error output
- `--verbose` - Show detailed output

## JSON Output

When `--json` is specified, all commands output structured JSON:

```bash
# Success
$ avfs ls --json /docs
{
  "path": "/docs",
  "entries": [...]
}

# Error
$ avfs cat --json /nonexistent
{
  "error": "NotFound",
  "message": "File not found: /nonexistent"
}
```

Exit codes: `0` for success, `1` for errors. Always check both exit code and JSON.
