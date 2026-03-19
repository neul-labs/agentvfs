# Agent Integration

This guide explains how to use avfs as a sandboxed filesystem for AI agents.

## Overview

avfs provides a safe, isolated filesystem for AI agents to:
- Read and write files without touching the real filesystem
- Experiment freely with snapshot/restore for rollback
- Operate within resource quotas to prevent runaway behavior
- Debug issues via audit logs

## Quick Start

```python
import subprocess
import json

class VFS:
    def __init__(self, vault="agent-workspace"):
        self.vault = vault

    def run(self, *args):
        """Execute a avfs command and return parsed JSON."""
        cmd = ["avfs", "--vault", self.vault, "--json"] + list(args)
        result = subprocess.run(cmd, capture_output=True, text=True)
        if result.returncode != 0:
            error = json.loads(result.stdout) if result.stdout else {"error": result.stderr}
            raise Exception(f"avfs error: {error}")
        return json.loads(result.stdout) if result.stdout else None

# Initialize
avfs = VFS("agent-workspace")

# Create workspace
avfs.run("mkdir", "/workspace")
avfs.run("write", "/workspace/hello.txt", "Hello, World!")

# Read back
result = avfs.run("cat", "/workspace/hello.txt")
print(result["content"])  # "Hello, World!"

# List files
result = avfs.run("ls", "/workspace")
for entry in result["entries"]:
    print(f"{entry['name']} ({entry['type']})")
```

## JSON Output Mode

All commands support `--json` for structured output that agents can parse.

### Successful Operations

```bash
# List directory
$ avfs ls --json /docs
{
  "path": "/docs",
  "entries": [
    {
      "name": "readme.txt",
      "type": "file",
      "size": 1234,
      "modified": "2024-03-10T14:22:15Z"
    },
    {
      "name": "src",
      "type": "directory",
      "modified": "2024-03-09T10:00:00Z"
    }
  ]
}

# Read file
$ avfs cat --json /docs/readme.txt
{
  "path": "/docs/readme.txt",
  "content": "Hello, World!",
  "size": 13,
  "version": 5,
  "modified": "2024-03-10T14:22:15Z"
}

# Write file
$ avfs write --json /docs/new.txt "content here"
{
  "path": "/docs/new.txt",
  "size": 12,
  "version": 1,
  "created": true
}

# Search
$ avfs grep --json "TODO" /src/
{
  "pattern": "TODO",
  "matches": [
    {
      "path": "/src/main.rs",
      "line": 42,
      "content": "// TODO: implement error handling"
    }
  ],
  "total": 1
}
```

### Error Responses

Errors return JSON with consistent structure:

```bash
$ avfs cat --json /nonexistent
{
  "error": "NotFound",
  "message": "File not found: /nonexistent",
  "path": "/nonexistent"
}

$ avfs write --json /docs/file.txt "content"
{
  "error": "QuotaExceeded",
  "message": "Vault would exceed max_size_mb limit (100 MB)",
  "current_size_mb": 98.5,
  "requested_size_mb": 5.2,
  "limit_mb": 100
}
```

**Exit codes:**
- `0` - Success
- `1` - Error (check JSON for details)

## Snapshots

Snapshots save the entire vault state, allowing agents to experiment and rollback.

### Basic Workflow

```bash
# Save state before experiment
$ avfs snapshot save before-experiment
{
  "name": "before-experiment",
  "created": "2024-03-10T14:00:00Z",
  "size_mb": 4.5
}

# Agent does work...
$ avfs mkdir /experiment
$ avfs write /experiment/code.py "print('test')"
$ avfs rm /important/file.txt  # Oops!

# Restore to saved state
$ avfs snapshot restore before-experiment
{
  "restored": "before-experiment",
  "previous_state": "auto-backup-1710072000"
}

# /important/file.txt is back, /experiment is gone
```

### Snapshot Commands

```bash
# List all snapshots
$ avfs snapshot list --json
{
  "snapshots": [
    {
      "name": "before-experiment",
      "created": "2024-03-10T14:00:00Z",
      "size_mb": 4.5
    },
    {
      "name": "initial",
      "created": "2024-03-10T10:00:00Z",
      "size_mb": 1.2
    }
  ]
}

# Delete a snapshot
$ avfs snapshot delete initial

# Create snapshot with auto-generated name
$ avfs snapshot save
{
  "name": "snapshot-1710072000",
  "created": "2024-03-10T14:00:00Z",
  "size_mb": 4.5
}
```

### Agent Pattern: Experiment Loop

```python
def experiment_safely(avfs, experiment_fn):
    """Run experiment with automatic rollback on failure."""
    # Save state
    snapshot = avfs.run("snapshot", "save")
    snapshot_name = snapshot["name"]

    try:
        result = experiment_fn()
        return {"success": True, "result": result}
    except Exception as e:
        # Rollback on failure
        avfs.run("snapshot", "restore", snapshot_name)
        return {"success": False, "error": str(e), "rolled_back": True}
    finally:
        # Clean up snapshot
        avfs.run("snapshot", "delete", snapshot_name)
```

## Quotas

Quotas prevent agents from consuming excessive resources.

### Setting Quotas

```bash
# Set limits
$ avfs vault config max_size_mb 100      # Total vault size
$ avfs vault config max_files 10000       # Maximum file count
$ avfs vault config max_file_size_mb 10   # Single file limit

# View current limits and usage
$ avfs vault info --json
{
  "name": "agent-workspace",
  "size_mb": 45.2,
  "file_count": 1234,
  "version_count": 5678,
  "limits": {
    "max_size_mb": 100,
    "max_files": 10000,
    "max_file_size_mb": 10
  },
  "usage": {
    "size_percent": 45.2,
    "files_percent": 12.34
  }
}
```

### Quota Errors

When quotas are exceeded, operations fail with clear errors:

```bash
$ avfs write --json /large-file "..."
{
  "error": "QuotaExceeded",
  "message": "File size (15 MB) exceeds max_file_size_mb (10 MB)",
  "type": "max_file_size_mb",
  "requested": 15,
  "limit": 10
}

$ avfs touch --json /another-file
{
  "error": "QuotaExceeded",
  "message": "File count (10000) would exceed max_files limit",
  "type": "max_files",
  "current": 10000,
  "limit": 10000
}
```

### Agent Pattern: Check Before Write

```python
def safe_write(avfs, path, content):
    """Write with quota awareness."""
    # Check current usage
    info = avfs.run("vault", "info")

    content_size_mb = len(content.encode()) / (1024 * 1024)
    limits = info.get("limits", {})

    # Check single file limit
    max_file = limits.get("max_file_size_mb", float("inf"))
    if content_size_mb > max_file:
        raise Exception(f"Content too large: {content_size_mb:.1f}MB > {max_file}MB limit")

    # Check total size limit
    max_total = limits.get("max_size_mb", float("inf"))
    current = info["size_mb"]
    if current + content_size_mb > max_total:
        raise Exception(f"Would exceed vault size limit: {current + content_size_mb:.1f}MB > {max_total}MB")

    return avfs.run("write", path, content)
```

## Audit Log

The audit log records all operations for debugging and analysis.

### Viewing the Log

```bash
# Recent operations (human-readable)
$ avfs audit
TIMESTAMP            OP       PATH                    RESULT
2024-03-10 14:22:15  write    /docs/readme.txt        ok
2024-03-10 14:22:10  mkdir    /docs                   ok
2024-03-10 14:22:05  rm       /old/file.txt           ok

# JSON output
$ avfs audit --json --limit 100
{
  "entries": [
    {
      "timestamp": "2024-03-10T14:22:15Z",
      "operation": "write",
      "path": "/docs/readme.txt",
      "result": "ok",
      "size": 1234,
      "version": 5
    }
  ],
  "total": 100,
  "has_more": true
}

# Filter by operation type
$ avfs audit --json --op write --op rm

# Filter by time range
$ avfs audit --json --since "2024-03-10T14:00:00Z"

# Filter by path prefix
$ avfs audit --json --path "/docs/"
```

### Audit Log Schema

Each entry contains:

| Field | Description |
|-------|-------------|
| `timestamp` | ISO 8601 timestamp |
| `operation` | Command name (write, rm, mkdir, etc.) |
| `path` | Primary path involved |
| `result` | "ok" or error type |
| `size` | Bytes written (for write ops) |
| `version` | Version created (for write ops) |
| `error` | Error message (if failed) |

### Agent Pattern: Debug Failed Operations

```python
def debug_last_failure(avfs):
    """Find what went wrong in recent operations."""
    audit = avfs.run("audit", "--limit", "50")

    failures = [e for e in audit["entries"] if e["result"] != "ok"]

    if not failures:
        print("No recent failures")
        return

    print(f"Found {len(failures)} failures:")
    for f in failures:
        print(f"  {f['timestamp']}: {f['operation']} {f['path']}")
        print(f"    Error: {f.get('error', 'unknown')}")
```

### Clearing the Log

```bash
# Clear all entries
$ avfs audit clear

# Audit log auto-rotates at 10k entries by default
$ avfs vault config audit_max_entries 50000
```

## Complete Agent Example

```python
#!/usr/bin/env python3
"""Example agent harness using avfs."""

import subprocess
import json
import sys

class VFSAgent:
    def __init__(self, vault_name="agent-workspace"):
        self.vault = vault_name
        self._setup_vault()

    def _setup_vault(self):
        """Create vault with appropriate limits."""
        # Create vault if needed
        try:
            self.run("vault", "create", self.vault)
        except:
            pass  # Already exists

        # Set conservative limits
        self.run("vault", "config", "max_size_mb", "100")
        self.run("vault", "config", "max_files", "1000")
        self.run("vault", "config", "max_file_size_mb", "5")

    def run(self, *args):
        """Execute avfs command, return parsed JSON."""
        cmd = ["avfs", "--vault", self.vault, "--json"] + [str(a) for a in args]
        result = subprocess.run(cmd, capture_output=True, text=True)

        if result.stdout:
            data = json.loads(result.stdout)
            if result.returncode != 0:
                raise Exception(f"VFS Error: {data.get('message', data)}")
            return data
        return None

    def checkpoint(self, name=None):
        """Save current state."""
        args = ["snapshot", "save"]
        if name:
            args.append(name)
        return self.run(*args)

    def rollback(self, name):
        """Restore to checkpoint."""
        return self.run("snapshot", "restore", name)

    def read(self, path):
        """Read file contents."""
        result = self.run("cat", path)
        return result["content"]

    def write(self, path, content):
        """Write file contents."""
        return self.run("write", path, content)

    def ls(self, path="/"):
        """List directory."""
        result = self.run("ls", path)
        return result["entries"]

    def mkdir(self, path):
        """Create directory."""
        return self.run("mkdir", "-p", path)

    def rm(self, path, recursive=False):
        """Remove file or directory."""
        args = ["rm"]
        if recursive:
            args.append("-r")
        args.append(path)
        return self.run(*args)

    def search(self, pattern, path="/"):
        """Search file contents."""
        result = self.run("grep", pattern, path)
        return result["matches"]

    def get_audit(self, limit=100):
        """Get recent operations."""
        return self.run("audit", "--limit", str(limit))["entries"]


def main():
    agent = VFSAgent("demo-agent")

    # Create checkpoint
    checkpoint = agent.checkpoint("start")
    print(f"Created checkpoint: {checkpoint['name']}")

    # Do some work
    agent.mkdir("/project/src")
    agent.write("/project/src/main.py", "print('Hello, World!')")
    agent.write("/project/README.md", "# My Project")

    # List what we created
    print("\nFiles created:")
    for entry in agent.ls("/project"):
        print(f"  {entry['name']} ({entry['type']})")

    # Show audit log
    print("\nRecent operations:")
    for entry in agent.get_audit(5):
        print(f"  {entry['operation']:10} {entry['path']}")

    # Rollback
    agent.rollback("start")
    print("\nRolled back to checkpoint")

    # Verify rollback
    try:
        agent.ls("/project")
        print("ERROR: /project still exists!")
    except:
        print("/project no longer exists (rollback successful)")


if __name__ == "__main__":
    main()
```

## Best Practices

### 1. Always Use JSON Mode
```python
# Good - parseable
result = subprocess.run(["avfs", "--json", "ls", "/"], capture_output=True)
data = json.loads(result.stdout)

# Bad - fragile text parsing
result = subprocess.run(["avfs", "ls", "/"], capture_output=True)
files = result.stdout.split("\n")  # Breaks on filenames with newlines
```

### 2. Checkpoint Before Risky Operations
```python
def risky_operation(agent):
    cp = agent.checkpoint()
    try:
        # Do risky stuff
        agent.rm("/", recursive=True)  # Yikes!
    except:
        agent.rollback(cp["name"])
        raise
```

### 3. Set Appropriate Quotas
```bash
# For code generation agents
avfs vault config max_size_mb 50
avfs vault config max_files 500
avfs vault config max_file_size_mb 1

# For data processing agents
avfs vault config max_size_mb 1000
avfs vault config max_files 10000
avfs vault config max_file_size_mb 100
```

### 4. Use Audit Logs for Debugging
```python
def diagnose_issue(agent):
    audit = agent.get_audit(100)

    # Find the problem
    errors = [e for e in audit if e["result"] != "ok"]
    writes = [e for e in audit if e["operation"] == "write"]

    print(f"Recent errors: {len(errors)}")
    print(f"Files written: {len(writes)}")
```

### 5. Clean Up Snapshots
```python
def cleanup_old_snapshots(agent, keep=5):
    snapshots = agent.run("snapshot", "list")["snapshots"]

    # Sort by date, delete oldest
    snapshots.sort(key=lambda s: s["created"])
    for snap in snapshots[:-keep]:
        agent.run("snapshot", "delete", snap["name"])
```
