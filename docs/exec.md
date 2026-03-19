# External Command Execution

vfs allows you to run external bash commands on virtual files, bridging the gap between the virtual filesystem and standard Unix tools.

## Overview

Two approaches are supported:

1. **Temp Extraction** (`vfs exec`): Extract file to temp, run command, re-import
2. **Pipe-Based** (`vfs cat | cmd | vfs write`): Stream through stdin/stdout

## Temp Extraction: `vfs exec`

The `exec` command temporarily extracts a file to the real filesystem, runs a command, and re-imports the result.

### Syntax

```bash
vfs exec [OPTIONS] '<COMMAND>' <PATH>...
```

### How It Works

1. File is extracted to a secure temp directory
2. Command runs with the temp file path substituted
3. Modified file is re-imported (creating a new version)
4. Temp file is securely deleted

### Single File Operations

```bash
# In-place text replacement with sed
vfs exec 'sed -i s/foo/bar/g' /docs/file.txt

# Format JSON with jq
vfs exec 'jq .' /config/settings.json

# Sort lines in a file
vfs exec 'sort -o {} {}' /data/names.txt

# Convert image format (requires ImageMagick)
vfs exec 'convert {} {}.png && mv {}.png {}' /images/photo.jpg
```

The `{}` placeholder is replaced with the temp file path. If omitted, the temp path is appended to the command.

### Options

- `-n, --no-reimport` - Run command but don't re-import the result
- `-v, --verbose` - Show temp file location and command
- `--dry-run` - Show what would be executed without running
- `-e, --env <KEY=VALUE>` - Set environment variable
- `--timeout <SECONDS>` - Kill command after timeout (default: 60)

### Glob Pattern Support

Execute commands on multiple files matching a pattern:

```bash
# Format all JSON files
vfs exec 'jq .' '/config/*.json'

# Convert all markdown to uppercase (example)
vfs exec 'tr a-z A-Z' '/docs/**/*.md'

# Process all log files
vfs exec 'gzip' '/logs/*.log'
```

Each matching file is processed sequentially. Use `--parallel` for concurrent execution:

```bash
vfs exec --parallel 4 'process_file' '/data/*.csv'
```

### Error Handling

If a command fails (non-zero exit code):
- The original file is NOT modified
- Error output is displayed
- Processing continues to next file (in glob mode)

Use `--fail-fast` to stop on first error:

```bash
vfs exec --fail-fast 'validate' '/src/*.rs'
```

### Rollback

Since every modification creates a new version, you can always rollback:

```bash
# Oops, wrong sed command!
vfs exec 'sed -i s/good/bad/g' /docs/important.txt

# Undo by reverting to previous version
vfs revert /docs/important.txt
```

## Pipe-Based Operations

For streaming data through commands, use the pipe approach with `cat` and `write`:

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

# Count lines and save
vfs cat /data/file.txt | wc -l | vfs write /stats/linecount.txt

# Multi-stage pipeline
vfs cat /data/raw.csv | \
    cut -d',' -f1,3 | \
    sort -t',' -k2 | \
    head -100 | \
    vfs write /data/processed.csv
```

### Reading from stdin

Write external data into the virtual filesystem:

```bash
# Pipe from real filesystem
cat ~/real-file.txt | vfs write /imported/file.txt

# Pipe from curl
curl -s https://api.example.com/data | vfs write /api/response.json

# Pipe from any command
date | vfs write /logs/timestamp.txt
```

### Writing to stdout

Export virtual file contents to external commands:

```bash
# View in pager
vfs cat /docs/readme.txt | less

# Open in editor (read-only view)
vfs cat /src/main.rs | vim -

# Send to clipboard (Linux)
vfs cat /notes/snippet.txt | xclip -selection clipboard

# Print with formatting
vfs cat /report.md | pandoc -t pdf > report.pdf
```

## Comparison: exec vs Pipe

| Feature | `vfs exec` | Pipe |
|---------|-----------|------|
| In-place editing | Yes | Yes (same input/output path) |
| Multiple files | Yes (globs) | One at a time |
| Command complexity | Any shell command | Pipeline chains |
| Temp file created | Yes | No |
| Streaming support | No | Yes |
| Large files | Memory-limited | Stream-friendly |
| Error rollback | Automatic | Manual |

### When to Use `exec`

- Commands that require file paths (not stdin/stdout)
- Commands that modify files in-place (like `sed -i`)
- Processing multiple files with globs
- When you need automatic rollback on error

### When to Use Pipes

- Streaming large files
- Complex multi-stage pipelines
- Commands designed for stdin/stdout
- When you want explicit control over input/output

## Security Considerations

### Sandboxing

Commands run with:
- Limited file access (only the temp directory)
- No network access by default
- Resource limits (CPU, memory, time)

Enable network access for specific commands:

```bash
vfs exec --allow-network 'curl-based-tool' /file
```

### Shell Injection Prevention

Commands are NOT passed through a shell by default. To use shell features:

```bash
# This runs the command directly (safer)
vfs exec 'my-tool --arg value' /file

# This uses shell interpretation (be careful!)
vfs exec --shell 'echo $HOME && my-tool' /file
```

### Temp Directory

Temp files are created in a secure directory:
- Mode 0700 (owner-only access)
- Random filename
- Deleted immediately after use
- Located in system temp or `--temp-dir` path

## Advanced Usage

### Chaining with Version Control

```bash
# Make changes and tag the version
vfs exec 'sed -i s/dev/prod/g' /config/app.yaml
vfs tag /config/app.yaml production-ready

# Later, find the tagged version
vfs log /config/app.yaml | grep production-ready
```

### Batch Processing Script

```bash
#!/bin/bash
# process_all.sh - Process all files in a vault directory

for file in $(vfs find /data -name "*.csv" -type f); do
    echo "Processing $file..."
    vfs exec 'csvclean' "$file"
done

echo "Done! Running compaction..."
vfs compact
```

### Integration with Make/Build Tools

```makefile
# Makefile example
build:
	vfs exec 'cargo build --release' /src/main.rs
	vfs export /target/release/myapp ./dist/

test:
	vfs exec 'cargo test' /src/lib.rs
```

## Troubleshooting

### Command Not Found

Ensure the command is in your PATH or use absolute paths:

```bash
vfs exec '/usr/local/bin/my-tool' /file
```

### Permission Denied

Check that the command has execute permissions:

```bash
vfs exec 'chmod +x ./script.sh && ./script.sh' /data
```

### Timeout Issues

Increase the timeout for long-running commands:

```bash
vfs exec --timeout 300 'slow-processor' /large-file
```

### Debug Mode

See exactly what's happening:

```bash
vfs exec -v --dry-run 'my-command' '/files/*.txt'
```
