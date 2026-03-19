# Import & Export

VFS provides commands to bridge between the virtual filesystem and the real filesystem, plus the ability to run external commands on virtual files.

## Import Files

Import files from the real filesystem into a vault.

### Basic Import

```bash
vfs import <real_path> <vfs_path>
```

**Examples:**

```bash
# Import a single file
vfs import ~/documents/report.pdf /docs/report.pdf

# Import to current directory
vfs import ~/data.csv /data.csv

# Import with different name
vfs import ~/old-name.txt /new-name.txt
```

### Recursive Import

Import entire directories:

```bash
vfs import -r ~/project /project
```

**Options:**

| Option | Description |
|--------|-------------|
| `-r, --recursive` | Import directories recursively |
| `--max-depth <N>` | Limit recursion depth |

**Examples:**

```bash
# Import directory tree
vfs import -r ~/src /src

# Limit depth
vfs import -r --max-depth 2 ~/project /project
```

## Export Files

Export files from a vault to the real filesystem.

### Basic Export

```bash
vfs export <vfs_path> <real_path>
```

**Examples:**

```bash
# Export a single file
vfs export /docs/report.pdf ~/downloads/report.pdf

# Export to current directory
vfs export /config.yaml ./config.yaml
```

### Recursive Export

Export entire directories:

```bash
vfs export -r /project ~/exported-project
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
vfs export -r --overwrite /backup ~/backup

# Export specific version
vfs export --version 3 /docs/readme.txt ~/old-readme.txt
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
vfs exec '<COMMAND>' <PATH>
```

**Examples:**

```bash
# In-place text replacement with sed
vfs exec 'sed -i s/foo/bar/g' /docs/file.txt

# Format JSON with jq
vfs exec 'jq .' /config/settings.json

# Sort lines in a file
vfs exec 'sort -o {} {}' /data/names.txt
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
vfs exec 'wc -l' /data/file.txt
```

## Pipe-Based Operations

For streaming data, use pipes with `cat` and `write`:

### Syntax

```bash
vfs cat <PATH> | <command> | vfs write <PATH>
```

### Examples

```bash
# Sort and deduplicate
vfs cat /data/names.txt | sort | uniq | vfs write /data/names-sorted.txt

# Filter log lines
vfs cat /logs/app.log | grep ERROR | vfs write /logs/errors.log

# Transform JSON
vfs cat /config/settings.json | jq '.debug = true' | vfs write /config/settings.json

# Compress content
vfs cat /data/large.txt | gzip | vfs write /data/large.txt.gz
```

### Reading from External Sources

```bash
# Pipe from real filesystem
cat ~/real-file.txt | vfs write /imported/file.txt

# Pipe from curl
curl -s https://api.example.com/data | vfs write /api/response.json

# Pipe from any command
date | vfs write /logs/timestamp.txt
```

### Writing to External Destinations

```bash
# View in pager
vfs cat /docs/readme.txt | less

# Send to clipboard (Linux)
vfs cat /notes/snippet.txt | xclip -selection clipboard

# Convert with pandoc
vfs cat /report.md | pandoc -t pdf > report.pdf
```

## exec vs Pipe Comparison

| Feature | `vfs exec` | Pipe |
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
vfs export -r / ~/vault-backup-$(date +%Y%m%d)
```

### Import Project Files

```bash
# Import a project
vfs import -r ~/code/myproject /myproject

# Verify
vfs tree /myproject
```

### Process Files with External Tools

```bash
# Format all JSON files
for file in $(vfs find / -n "*.json"); do
    vfs exec 'jq .' "$file"
done

# Or using cat/write for complex pipelines
vfs cat /data.csv | \
    cut -d',' -f1,3 | \
    sort -t',' -k2 | \
    head -100 | \
    vfs write /data-processed.csv
```
