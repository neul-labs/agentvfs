# File Operations

VFS provides familiar file operations that work just like their Unix counterparts.

## Listing Files

### ls - List Directory Contents

```bash
vfs ls [OPTIONS] [PATH]
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
vfs ls                    # List current directory
vfs ls /docs              # List specific directory
vfs ls -l /docs           # Detailed listing
vfs ls -laR /             # Full recursive listing
```

### tree - Display Directory Tree

```bash
vfs tree [OPTIONS] [PATH]
```

**Options:**

| Option | Description |
|--------|-------------|
| `-L, --level <N>` | Limit depth to N levels |
| `-d, --dirs-only` | Show only directories |
| `--size` | Show file sizes |

**Examples:**

```bash
vfs tree                  # Tree from root
vfs tree -L 2 /           # Tree with max depth 2
vfs tree --size /docs     # Tree with file sizes
```

### pwd - Print Working Directory

```bash
vfs pwd
```

In VFS, the working directory is always `/` unless you're in the interactive shell.

## Reading Files

### cat - Display File Contents

```bash
vfs cat [OPTIONS] <PATH>...
```

**Options:**

| Option | Description |
|--------|-------------|
| `-n, --number` | Number output lines |
| `-v, --version <N>` | Show specific version |

**Examples:**

```bash
vfs cat /docs/readme.txt          # Show file contents
vfs cat -n /src/main.rs           # With line numbers
vfs cat -v 3 /docs/readme.txt     # Show version 3
vfs cat file1.txt file2.txt       # Concatenate multiple files
```

## Writing Files

### write - Write Content to a File

```bash
vfs write [OPTIONS] <PATH> [CONTENT]
```

**Options:**

| Option | Description |
|--------|-------------|
| `-a, --append` | Append instead of overwrite |
| `--stdin` | Read content from stdin |

**Examples:**

```bash
# Write content directly
vfs write /docs/new.txt "Hello, World!"

# Append to existing file
vfs write -a /docs/log.txt "New line"

# From stdin
echo "piped content" | vfs write --stdin /docs/file.txt

# Multi-line content
vfs write /docs/multi.txt "Line 1
Line 2
Line 3"
```

!!! tip "Version Created"
    Every write operation creates a new version of the file, enabling rollback.

## Copying Files

### cp - Copy Files or Directories

```bash
vfs cp [OPTIONS] <SOURCE>... <DEST>
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
vfs cp /docs/readme.txt /docs/backup.txt

# Copy multiple files to a directory
vfs cp /src/a.txt /src/b.txt /backup/

# Copy directory recursively
vfs cp -r /docs /docs-backup

# Copy with rename
vfs cp /config.txt /config.txt.bak
```

## Moving and Renaming

### mv - Move or Rename Files

```bash
vfs mv [OPTIONS] <SOURCE>... <DEST>
```

**Options:**

| Option | Description |
|--------|-------------|
| `-f, --force` | Overwrite without prompting |
| `-n, --no-clobber` | Don't overwrite existing files |

**Examples:**

```bash
# Rename a file
vfs mv /docs/old.txt /docs/new.txt

# Move file to directory
vfs mv /readme.txt /docs/

# Move multiple files
vfs mv /file1.txt /file2.txt /archive/

# Move and rename
vfs mv /src/main.rs /backup/main.rs.bak
```

## Creating Directories

### mkdir - Create Directories

```bash
vfs mkdir [OPTIONS] <PATH>...
```

**Options:**

| Option | Description |
|--------|-------------|
| `-p, --parents` | Create parent directories as needed |

**Examples:**

```bash
# Create a directory
vfs mkdir /docs

# Create nested directories
vfs mkdir -p /src/components/ui

# Create multiple directories
vfs mkdir /logs /temp /cache
```

## Removing Files and Directories

### rm - Remove Files or Directories

```bash
vfs rm [OPTIONS] <PATH>...
```

**Options:**

| Option | Description |
|--------|-------------|
| `-r, --recursive` | Remove directories and their contents |
| `-f, --force` | Don't prompt for confirmation |

**Examples:**

```bash
# Remove a file
vfs rm /docs/old.txt

# Remove multiple files
vfs rm /temp/a.txt /temp/b.txt

# Remove empty directory
vfs rm /empty-dir

# Remove directory with contents
vfs rm -r /old-project

# Force remove without confirmation
vfs rm -rf /temp/
```

!!! warning "Permanent Deletion"
    Removed files are deleted from the filesystem. However, content may still exist
    in the blob store until garbage collection runs. Use [versioning](versioning.md)
    to recover recent files before they're pruned.

## Working with Stdin/Stdout

### Piping Content

```bash
# Write from another command
echo "Generated content" | vfs write --stdin /generated.txt

# Read and pipe to another command
vfs cat /data.json | jq '.items[]'

# Copy between files using pipes
vfs cat /source.txt | vfs write --stdin /dest.txt
```

### Combining with Unix Tools

```bash
# Count lines
vfs cat /data.txt | wc -l

# Sort content
vfs cat /names.txt | sort

# Filter content
vfs cat /log.txt | grep "ERROR"

# Transform and save
vfs cat /data.csv | sed 's/,/|/g' | vfs write --stdin /data.psv
```

## JSON Output

All file operations support JSON output for scripting:

```bash
# List with JSON
vfs ls --json /docs

# File info in JSON
vfs cat --json /docs/readme.txt
```

## Best Practices

### Organize with Directories

```bash
# Create a project structure
vfs mkdir -p /project/{src,docs,tests,config}
```

### Use Meaningful Names

```bash
# Good
vfs write /docs/user-guide.md "..."
vfs write /config/database.yaml "..."

# Avoid
vfs write /d/ug.md "..."
vfs write /c/db.y "..."
```

### Regular Cleanup

```bash
# Remove temporary files
vfs rm -r /temp/*

# Archive old files
vfs mv /logs/*.log /archive/logs/
```
