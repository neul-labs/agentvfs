# File Operations

VFS provides familiar file operations that work just like their Unix counterparts.

## Listing Files

### ls - List Directory Contents

```bash
avfs ls [OPTIONS] [PATH]
```

**Options:**

| Option | Description |
|--------|-------------|
| `-l, --long` | Show detailed information (size, date, type) |
| `-a, --all` | Include hidden files (starting with `.`) |
| `-R, --recursive` | List subdirectories recursively |
| `-t, --time` | Sort by modification time |
| `-S, --size` | Sort by size |

**Examples:**

```bash
avfs ls                    # List current directory
avfs ls /docs              # List specific directory
avfs ls -l /docs           # Detailed listing
avfs ls -laR /             # Full recursive listing
```

### tree - Display Directory Tree

```bash
avfs tree [OPTIONS] [PATH]
```

**Options:**

| Option | Description |
|--------|-------------|
| `-L, --level <N>` | Limit depth to N levels |
| `-d, --dirs-only` | Show only directories |
| `--size` | Show file sizes |

**Examples:**

```bash
avfs tree                  # Tree from root
avfs tree -L 2 /           # Tree with max depth 2
avfs tree --size /docs     # Tree with file sizes
```

### pwd - Print Working Directory

```bash
avfs pwd
```

In VFS, the working directory is always `/` unless you're in the interactive shell.

## Reading Files

### cat - Display File Contents

```bash
avfs cat [OPTIONS] <PATH>...
```

**Options:**

| Option | Description |
|--------|-------------|
| `-n, --number` | Number output lines |
| `-v, --version <N>` | Show specific version |

**Examples:**

```bash
avfs cat /docs/readme.txt          # Show file contents
avfs cat -n /src/main.rs           # With line numbers
avfs cat -v 3 /docs/readme.txt     # Show version 3
avfs cat file1.txt file2.txt       # Concatenate multiple files
```

## Writing Files

### write - Write Content to a File

```bash
avfs write [OPTIONS] <PATH> [CONTENT]
```

**Options:**

| Option | Description |
|--------|-------------|
| `-a, --append` | Append instead of overwrite |
| `--stdin` | Read content from stdin |

**Examples:**

```bash
# Write content directly
avfs write /docs/new.txt "Hello, World!"

# Append to existing file
avfs write -a /docs/log.txt "New line"

# From stdin
echo "piped content" | avfs write --stdin /docs/file.txt

# Multi-line content
avfs write /docs/multi.txt "Line 1
Line 2
Line 3"
```

!!! tip "Version Created"
    Every write operation creates a new version of the file, enabling rollback.

## Copying Files

### cp - Copy Files or Directories

```bash
avfs cp [OPTIONS] <SOURCE>... <DEST>
```

**Options:**

| Option | Description |
|--------|-------------|
| `-r, --recursive` | Copy directories recursively |
| `-f, --force` | Overwrite without prompting |
| `-n, --no-clobber` | Don't overwrite existing files |

**Examples:**

```bash
# Copy a file
avfs cp /docs/readme.txt /docs/backup.txt

# Copy multiple files to a directory
avfs cp /src/a.txt /src/b.txt /backup/

# Copy directory recursively
avfs cp -r /docs /docs-backup

# Copy with rename
avfs cp /config.txt /config.txt.bak
```

## Moving and Renaming

### mv - Move or Rename Files

```bash
avfs mv [OPTIONS] <SOURCE>... <DEST>
```

**Options:**

| Option | Description |
|--------|-------------|
| `-f, --force` | Overwrite without prompting |
| `-n, --no-clobber` | Don't overwrite existing files |

**Examples:**

```bash
# Rename a file
avfs mv /docs/old.txt /docs/new.txt

# Move file to directory
avfs mv /readme.txt /docs/

# Move multiple files
avfs mv /file1.txt /file2.txt /archive/

# Move and rename
avfs mv /src/main.rs /backup/main.rs.bak
```

## Creating Directories

### mkdir - Create Directories

```bash
avfs mkdir [OPTIONS] <PATH>...
```

**Options:**

| Option | Description |
|--------|-------------|
| `-p, --parents` | Create parent directories as needed |

**Examples:**

```bash
# Create a directory
avfs mkdir /docs

# Create nested directories
avfs mkdir -p /src/components/ui

# Create multiple directories
avfs mkdir /logs /temp /cache
```

## Removing Files and Directories

### rm - Remove Files or Directories

```bash
avfs rm [OPTIONS] <PATH>...
```

**Options:**

| Option | Description |
|--------|-------------|
| `-r, --recursive` | Remove directories and their contents |
| `-f, --force` | Don't prompt for confirmation |

**Examples:**

```bash
# Remove a file
avfs rm /docs/old.txt

# Remove multiple files
avfs rm /temp/a.txt /temp/b.txt

# Remove empty directory
avfs rm /empty-dir

# Remove directory with contents
avfs rm -r /old-project

# Force remove without confirmation
avfs rm -rf /temp/
```

!!! warning "Permanent Deletion"
    Removed files are deleted from the filesystem. However, content may still exist
    in the blob store until garbage collection runs. Use [versioning](versioning.md)
    to recover recent files before they're pruned.

## Working with Stdin/Stdout

### Piping Content

```bash
# Write from another command
echo "Generated content" | avfs write --stdin /generated.txt

# Read and pipe to another command
avfs cat /data.json | jq '.items[]'

# Copy between files using pipes
avfs cat /source.txt | avfs write --stdin /dest.txt
```

### Combining with Unix Tools

```bash
# Count lines
avfs cat /data.txt | wc -l

# Sort content
avfs cat /names.txt | sort

# Filter content
avfs cat /log.txt | grep "ERROR"

# Transform and save
avfs cat /data.csv | sed 's/,/|/g' | avfs write --stdin /data.psv
```

## JSON Output

All file operations support JSON output for scripting:

```bash
# List with JSON
avfs ls --json /docs

# File info in JSON
avfs cat --json /docs/readme.txt
```

## Best Practices

### Organize with Directories

```bash
# Create a project structure
avfs mkdir -p /project/{src,docs,tests,config}
```

### Use Meaningful Names

```bash
# Good
avfs write /docs/user-guide.md "..."
avfs write /config/database.yaml "..."

# Avoid
avfs write /d/ug.md "..."
avfs write /c/db.y "..."
```

### Regular Cleanup

```bash
# Remove temporary files
avfs rm -r /temp/*

# Archive old files
avfs mv /logs/*.log /archive/logs/
```
