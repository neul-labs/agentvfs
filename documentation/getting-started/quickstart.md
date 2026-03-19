# Quick Start

This guide will get you up and running with VFS in about 5 minutes.

## Create Your First Vault

A **vault** is an independent virtual filesystem stored in a single database file.

```bash
# Create a new vault called "myproject"
avfs vault create myproject
```

VFS automatically switches to your new vault. You can verify with:

```bash
avfs vault list
```

Output:
```
* myproject    ~/.avfs/myproject.avfs    (current)
```

## Basic File Operations

### Create Directories

```bash
# Create a directory
avfs mkdir /docs

# Create nested directories with -p
avfs mkdir -p /src/components/ui
```

### Write Files

```bash
# Write content to a file
avfs write /docs/readme.txt "Hello, Virtual World!"

# Append to a file
avfs write -a /docs/readme.txt "Another line"
```

### List Contents

```bash
# Simple listing
avfs ls /docs

# Detailed listing with sizes and dates
avfs ls -l /docs

# Tree view
avfs tree /
```

### Read Files

```bash
# Display file contents
avfs cat /docs/readme.txt

# With line numbers
avfs cat -n /docs/readme.txt
```

### Copy and Move

```bash
# Copy a file
avfs cp /docs/readme.txt /docs/backup.txt

# Move/rename a file
avfs mv /docs/backup.txt /archive/readme-old.txt

# Copy a directory recursively
avfs cp -r /docs /docs-backup
```

### Delete

```bash
# Remove a file
avfs rm /archive/readme-old.txt

# Remove a directory (must be empty)
avfs rm /empty-dir

# Remove recursively
avfs rm -r /docs-backup
```

## Version History

Every time you modify a file, VFS automatically creates a new version.

```bash
# Make some changes
avfs write /docs/readme.txt "Version 1"
avfs write /docs/readme.txt "Version 2"
avfs write /docs/readme.txt "Version 3"

# View version history
avfs log /docs/readme.txt
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
avfs cat -v 1 /docs/readme.txt

# Restore to a previous version
avfs checkout /docs/readme.txt -v 1

# Revert to the immediately previous version
avfs revert /docs/readme.txt
```

## Search

### Full-Text Search

```bash
# Search all files for a term
avfs search "hello"
```

### Regex Search (Grep)

```bash
# Search with regex patterns
avfs grep "TODO|FIXME" /src/

# Case-insensitive search
avfs grep -i "error" /logs/
```

### Find Files

```bash
# Find by name pattern
avfs find -n "*.txt"

# Find by type (f=file, d=directory)
avfs find -t f /docs/

# Find files with a specific tag
avfs find --tag important
```

## Tags and Metadata

### Add Tags

```bash
# Tag a file
avfs tag /docs/readme.txt important

# List tags on a file
avfs tag /docs/readme.txt --list
```

### Custom Metadata

```bash
# Set metadata
avfs meta /docs/readme.txt author "John Doe"
avfs meta /docs/readme.txt status "draft"

# Get metadata
avfs meta /docs/readme.txt author

# List all metadata
avfs meta /docs/readme.txt
```

## Interactive Shell

For extended sessions, use the interactive shell:

```bash
avfs shell
```

In the shell, commands work without the `avfs` prefix:

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
avfs vault create another-project

# Switch vaults
avfs vault use myproject

# List all vaults
avfs vault list
```

## JSON Output

All commands support JSON output for scripting:

```bash
avfs ls --json /docs
avfs cat --json /docs/readme.txt
```

## What's Next?

- [Core Concepts](concepts.md) - Deeper understanding of VFS
- [File Operations](../user-guide/file-operations.md) - Complete file operations guide
- [Versioning](../user-guide/versioning.md) - Master version control
- [Command Reference](../reference/commands.md) - All commands documented
