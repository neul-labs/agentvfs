# External Command Execution

avfs allows you to run external bash commands on virtual files, bridging the gap between the virtual filesystem and standard Unix tools.

## Overview

Two approaches are supported:

1. **Temp Extraction** (`avfs exec`): Extract file to temp, run command, re-import
2. **Pipe-Based** (`avfs cat | cmd | avfs write`): Stream through stdin/stdout

## Temp Extraction: `avfs exec`

The `exec` command temporarily extracts a file to the real filesystem, runs a command, and re-imports the result.

### Syntax

```bash
avfs exec [OPTIONS] '<COMMAND>' <PATH>...
```

### How It Works

1. File is extracted to a secure temp directory
2. Command runs with the temp file path substituted
3. Modified file is re-imported (creating a new version)
4. Temp file is securely deleted

### Single File Operations

```bash
# In-place text replacement with sed
avfs exec 'sed -i s/foo/bar/g' /docs/file.txt

# Format JSON with jq
avfs exec 'jq .' /config/settings.json

# Sort lines in a file
avfs exec 'sort -o {} {}' /data/names.txt

# Convert image format (requires ImageMagick)
avfs exec 'convert {} {}.png && mv {}.png {}' /images/photo.jpg
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
avfs exec 'jq .' '/config/*.json'

# Convert all markdown to uppercase (example)
avfs exec 'tr a-z A-Z' '/docs/**/*.md'

# Process all log files
avfs exec 'gzip' '/logs/*.log'
```

Each matching file is processed sequentially. Use `--parallel` for concurrent execution:

```bash
avfs exec --parallel 4 'process_file' '/data/*.csv'
```

### Error Handling

If a command fails (non-zero exit code):
- The original file is NOT modified
- Error output is displayed
- Processing continues to next file (in glob mode)

Use `--fail-fast` to stop on first error:

```bash
avfs exec --fail-fast 'validate' '/src/*.rs'
```

### Rollback

Since every modification creates a new version, you can always rollback:

```bash
# Oops, wrong sed command!
avfs exec 'sed -i s/good/bad/g' /docs/important.txt

# Undo by reverting to previous version
avfs revert /docs/important.txt
```

## Pipe-Based Operations

For streaming data through commands, use the pipe approach with `cat` and `write`:

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

# Count lines and save
avfs cat /data/file.txt | wc -l | avfs write /stats/linecount.txt

# Multi-stage pipeline
avfs cat /data/raw.csv | \
    cut -d',' -f1,3 | \
    sort -t',' -k2 | \
    head -100 | \
    avfs write /data/processed.csv
```

### Reading from stdin

Write external data into the virtual filesystem:

```bash
# Pipe from real filesystem
cat ~/real-file.txt | avfs write /imported/file.txt

# Pipe from curl
curl -s https://api.example.com/data | avfs write /api/response.json

# Pipe from any command
date | avfs write /logs/timestamp.txt
```

### Writing to stdout

Export virtual file contents to external commands:

```bash
# View in pager
avfs cat /docs/readme.txt | less

# Open in editor (read-only view)
avfs cat /src/main.rs | vim -

# Send to clipboard (Linux)
avfs cat /notes/snippet.txt | xclip -selection clipboard

# Print with formatting
avfs cat /report.md | pandoc -t pdf > report.pdf
```

## Comparison: exec vs Pipe

| Feature | `avfs exec` | Pipe |
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
avfs exec --allow-network 'curl-based-tool' /file
```

### Shell Injection Prevention

Commands are NOT passed through a shell by default. To use shell features:

```bash
# This runs the command directly (safer)
avfs exec 'my-tool --arg value' /file

# This uses shell interpretation (be careful!)
avfs exec --shell 'echo $HOME && my-tool' /file
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
avfs exec 'sed -i s/dev/prod/g' /config/app.yaml
avfs tag /config/app.yaml production-ready

# Later, find the tagged version
avfs log /config/app.yaml | grep production-ready
```

### Batch Processing Script

```bash
#!/bin/bash
# process_all.sh - Process all files in a vault directory

for file in $(avfs find /data -name "*.csv" -type f); do
    echo "Processing $file..."
    avfs exec 'csvclean' "$file"
done

echo "Done! Running compaction..."
avfs compact
```

### Integration with Make/Build Tools

```makefile
# Makefile example
build:
	avfs exec 'cargo build --release' /src/main.rs
	avfs export /target/release/myapp ./dist/

test:
	avfs exec 'cargo test' /src/lib.rs
```

## Troubleshooting

### Command Not Found

Ensure the command is in your PATH or use absolute paths:

```bash
avfs exec '/usr/local/bin/my-tool' /file
```

### Permission Denied

Check that the command has execute permissions:

```bash
avfs exec 'chmod +x ./script.sh && ./script.sh' /data
```

### Timeout Issues

Increase the timeout for long-running commands:

```bash
avfs exec --timeout 300 'slow-processor' /large-file
```

### Debug Mode

See exactly what's happening:

```bash
avfs exec -v --dry-run 'my-command' '/files/*.txt'
```
