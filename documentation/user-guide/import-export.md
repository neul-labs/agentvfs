# Import & Export

VFS provides commands to bridge between the virtual filesystem and the real filesystem, plus the ability to run external commands on virtual files.

## Import Files

Import files from the real filesystem into a vault.

### Basic Import

```bash
avfs import <real_path> <vfs_path>
```

**Examples:**

```bash
# Import a single file
avfs import ~/documents/report.pdf /docs/report.pdf

# Import to current directory
avfs import ~/data.csv /data.csv

# Import with different name
avfs import ~/old-name.txt /new-name.txt
```

### Recursive Import

Import entire directories:

```bash
avfs import -r ~/project /project
```

**Options:**

| Option | Description |
|--------|-------------|
| `-r, --recursive` | Import directories recursively |
| `--max-depth <N>` | Limit recursion depth |

**Examples:**

```bash
# Import directory tree
avfs import -r ~/src /src

# Limit depth
avfs import -r --max-depth 2 ~/project /project
```

## Export Files

Export files from a vault to the real filesystem.

### Basic Export

```bash
avfs export <vfs_path> <real_path>
```

**Examples:**

```bash
# Export a single file
avfs export /docs/report.pdf ~/downloads/report.pdf

# Export to current directory
avfs export /config.yaml ./config.yaml
```

### Recursive Export

Export entire directories:

```bash
avfs export -r /project ~/exported-project
```

**Options:**

| Option | Description |
|--------|-------------|
| `-r, --recursive` | Export directories recursively |
| `--max-depth <N>` | Limit recursion depth |
| `--overwrite` | Overwrite existing files |

**Examples:**

```bash
# Export with overwrite
avfs export -r --overwrite /backup ~/backup

# Export specific version
avfs export --version 3 /docs/readme.txt ~/old-readme.txt
```

## External Commands (exec)

Run external bash commands on virtual files.

### How It Works

1. File is extracted to a secure temp directory
2. Command runs with the temp file
3. Modified file is re-imported (creating a new version)
4. Temp file is deleted

### Basic Usage

```bash
avfs exec '<COMMAND>' <PATH>
```

**Examples:**

```bash
# In-place text replacement with sed
avfs exec 'sed -i s/foo/bar/g' /docs/file.txt

# Format JSON with jq
avfs exec 'jq .' /config/settings.json

# Sort lines in a file
avfs exec 'sort -o {} {}' /data/names.txt
```

The `{}` placeholder is replaced with the temp file path.

### Options

| Option | Description |
|--------|-------------|
| `--reimport` | Re-import the modified file (default) |
| `--stdin` | Pass file content via stdin |

### No Re-import

Run command without modifying the virtual file:

```bash
# Just view the output
avfs exec 'wc -l' /data/file.txt
```

## Pipe-Based Operations

For streaming data, use pipes with `cat` and `write`:

### Syntax

```bash
avfs cat <PATH> | <command> | avfs write <PATH>
```

### Examples

```bash
# Sort and deduplicate
avfs cat /data/names.txt | sort | uniq | avfs write /data/names-sorted.txt

# Filter log lines
avfs cat /logs/app.log | grep ERROR | avfs write /logs/errors.log

# Transform JSON
avfs cat /config/settings.json | jq '.debug = true' | avfs write /config/settings.json

# Compress content
avfs cat /data/large.txt | gzip | avfs write /data/large.txt.gz
```

### Reading from External Sources

```bash
# Pipe from real filesystem
cat ~/real-file.txt | avfs write /imported/file.txt

# Pipe from curl
curl -s https://api.example.com/data | avfs write /api/response.json

# Pipe from any command
date | avfs write /logs/timestamp.txt
```

### Writing to External Destinations

```bash
# View in pager
avfs cat /docs/readme.txt | less

# Send to clipboard (Linux)
avfs cat /notes/snippet.txt | xclip -selection clipboard

# Convert with pandoc
avfs cat /report.md | pandoc -t pdf > report.pdf
```

## exec vs Pipe Comparison

| Feature | `avfs exec` | Pipe |
|---------|-----------|------|
| In-place editing | Yes | Yes |
| Multiple files | With globs | One at a time |
| Temp file created | Yes | No |
| Streaming support | No | Yes |
| Large files | Memory-limited | Stream-friendly |

### When to Use exec

- Commands that require file paths (not stdin/stdout)
- Commands that modify files in-place (like `sed -i`)
- When you need automatic versioning on modification

### When to Use Pipes

- Streaming large files
- Complex multi-stage pipelines
- Commands designed for stdin/stdout

## Common Workflows

### Backup to Real Filesystem

```bash
# Export entire vault
avfs export -r / ~/vault-backup-$(date +%Y%m%d)
```

### Import Project Files

```bash
# Import a project
avfs import -r ~/code/myproject /myproject

# Verify
avfs tree /myproject
```

### Process Files with External Tools

```bash
# Format all JSON files
for file in $(avfs find / -n "*.json"); do
    avfs exec 'jq .' "$file"
done

# Or using cat/write for complex pipelines
avfs cat /data.csv | \
    cut -d',' -f1,3 | \
    sort -t',' -k2 | \
    head -100 | \
    avfs write /data-processed.csv
```
