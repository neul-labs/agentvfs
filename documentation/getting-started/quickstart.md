# Quick Start

This guide will get you up and running with VFS in about 5 minutes.

## Create Your First Vault

A **vault** is an independent virtual filesystem stored in a single database file.

```bash
# Create a new vault called "myproject"
vfs vault create myproject
```

VFS automatically switches to your new vault. You can verify with:

```bash
vfs vault list
```

Output:
```
* myproject    ~/.vfs/myproject.vfs    (current)
```

## Basic File Operations

### Create Directories

```bash
# Create a directory
vfs mkdir /docs

# Create nested directories with -p
vfs mkdir -p /src/components/ui
```

### Write Files

```bash
# Write content to a file
vfs write /docs/readme.txt "Hello, Virtual World!"

# Append to a file
vfs write -a /docs/readme.txt "Another line"
```

### List Contents

```bash
# Simple listing
vfs ls /docs

# Detailed listing with sizes and dates
vfs ls -l /docs

# Tree view
vfs tree /
```

### Read Files

```bash
# Display file contents
vfs cat /docs/readme.txt

# With line numbers
vfs cat -n /docs/readme.txt
```

### Copy and Move

```bash
# Copy a file
vfs cp /docs/readme.txt /docs/backup.txt

# Move/rename a file
vfs mv /docs/backup.txt /archive/readme-old.txt

# Copy a directory recursively
vfs cp -r /docs /docs-backup
```

### Delete

```bash
# Remove a file
vfs rm /archive/readme-old.txt

# Remove a directory (must be empty)
vfs rm /empty-dir

# Remove recursively
vfs rm -r /docs-backup
```

## Version History

Every time you modify a file, VFS automatically creates a new version.

```bash
# Make some changes
vfs write /docs/readme.txt "Version 1"
vfs write /docs/readme.txt "Version 2"
vfs write /docs/readme.txt "Version 3"

# View version history
vfs log /docs/readme.txt
```

Output:
```
Version 3 (current) - 2024-01-15 10:32:45
Version 2           - 2024-01-15 10:32:30
Version 1           - 2024-01-15 10:32:15
```

### Restore Previous Versions

```bash
# Read a specific version
vfs cat -v 1 /docs/readme.txt

# Restore to a previous version
vfs checkout /docs/readme.txt -v 1

# Revert to the immediately previous version
vfs revert /docs/readme.txt
```

## Search

### Full-Text Search

```bash
# Search all files for a term
vfs search "hello"
```

### Regex Search (Grep)

```bash
# Search with regex patterns
vfs grep "TODO|FIXME" /src/

# Case-insensitive search
vfs grep -i "error" /logs/
```

### Find Files

```bash
# Find by name pattern
vfs find -n "*.txt"

# Find by type (f=file, d=directory)
vfs find -t f /docs/

# Find files with a specific tag
vfs find --tag important
```

## Tags and Metadata

### Add Tags

```bash
# Tag a file
vfs tag /docs/readme.txt important

# List tags on a file
vfs tag /docs/readme.txt --list
```

### Custom Metadata

```bash
# Set metadata
vfs meta /docs/readme.txt author "John Doe"
vfs meta /docs/readme.txt status "draft"

# Get metadata
vfs meta /docs/readme.txt author

# List all metadata
vfs meta /docs/readme.txt
```

## Interactive Shell

For extended sessions, use the interactive shell:

```bash
vfs shell
```

In the shell, commands work without the `vfs` prefix:

```
[vault:myproject] / > ls
docs/
src/
[vault:myproject] / > cd docs
[vault:myproject] /docs > cat readme.txt
Hello, Virtual World!
[vault:myproject] /docs > exit
```

## Switch Between Vaults

```bash
# Create another vault
vfs vault create another-project

# Switch vaults
vfs vault use myproject

# List all vaults
vfs vault list
```

## JSON Output

All commands support JSON output for scripting:

```bash
vfs ls --json /docs
vfs cat --json /docs/readme.txt
```

## What's Next?

- [Core Concepts](concepts.md) - Deeper understanding of VFS
- [File Operations](../user-guide/file-operations.md) - Complete file operations guide
- [Versioning](../user-guide/versioning.md) - Master version control
- [Command Reference](../reference/commands.md) - All commands documented
