# External Command Execution

avfs allows you to run external commands on virtual files by extracting them to a temporary file on the host filesystem.

## Overview

The `exec` command temporarily extracts a file to the real filesystem, runs a command, and optionally re-imports the result.

## Syntax

```bash
avfs exec [OPTIONS] <VFS_PATH> -- <COMMAND>...
```

## How It Works

1. File content is read from the vault and written to a secure temp file
2. The command runs with the temp file path substituted
3. If `--reimport` is set, the temp file is read back and written to the vault as a new version
4. Temp file is deleted

## Options

- `--reimport` - Read the temp file back into the vault after the command completes
- `--stdin` - Pass vault file content via stdin instead of as a temp file argument

## Placeholder

Use `{}` in the command to substitute the temp file path:

```bash
# In-place text replacement with sed
avfs exec --reimport /docs/file.txt -- sed -i s/foo/bar/g {}

# Convert image format (requires ImageMagick)
avfs exec --reimport /images/photo.jpg -- convert {} {}.png && mv {}.png {}
```

If `{}` is not used, the temp file path is appended to the command arguments.

## Examples

### Process and re-import

```bash
# Format JSON with jq
avfs exec --reimport /config/settings.json -- jq . {}

# Sort lines in a file
avfs exec --reimport /data/names.txt -- sort -o {} {}
```

### Read-only analysis

```bash
# Count words without modifying the vault
avfs exec /docs/report.md -- wc -w

# Search for patterns
avfs exec /src/main.rs -- grep "TODO"
```

### Stdin mode

```bash
# Pass content via stdin instead of temp file
avfs exec --stdin /data/large.txt -- gzip > output.gz
```

## Output

When run with `--json`, returns structured data:

```json
{
  "vfs_path": "/docs/file.txt",
  "command": "sed -i s/foo/bar/g /tmp/.tmpABC123",
  "exit_code": 0,
  "stdout": "",
  "stderr": "",
  "reimported": true,
  "reimport_bytes": 1234
}
```

## Error Handling

If the external command exits with a non-zero status, avfs returns the exit code and does **not** re-import the file (even if `--reimport` was requested).

## Security Considerations

- Commands run with the privileges of the avfs process
- Temp files are created in the system temp directory
- Always validate commands before running them in automated workflows

## See Also

- [`proxy exec`](proxy.md) - Policy-gated execution inside a mounted workspace
