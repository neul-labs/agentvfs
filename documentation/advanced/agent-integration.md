# Agent Integration

This guide explains how to use VFS as a sandboxed filesystem for AI agents.

## Overview

VFS provides a safe, isolated filesystem for AI agents to:

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
        """Execute a vfs command and return parsed JSON."""
        cmd = ["vfs", "--vault", self.vault, "--json"] + list(args)
        result = subprocess.run(cmd, capture_output=True, text=True)
        if result.returncode != 0:
            error = json.loads(result.stdout) if result.stdout else {"error": result.stderr}
            raise Exception(f"vfs error: {error}")
        return json.loads(result.stdout) if result.stdout else None

# Initialize
vfs = VFS("agent-workspace")

# Create workspace
vfs.run("mkdir", "/workspace")
vfs.run("write", "/workspace/hello.txt", "Hello, World!")

# Read back
result = vfs.run("cat", "/workspace/hello.txt")
print(result["content"])  # "Hello, World!"
```

## JSON Output Mode

All commands support `--json` for structured output that agents can parse.

### Successful Operations

```bash
# List directory
$ vfs ls --json /docs
{
  "path": "/docs",
  "entries": [
    {
      "name": "readme.txt",
      "type": "file",
      "size": 1234,
      "modified": "2024-03-10T14:22:15Z"
    }
  ]
}

# Read file
$ vfs cat --json /docs/readme.txt
{
  "path": "/docs/readme.txt",
  "content": "Hello, World!",
  "size": 13,
  "version": 5
}
```

### Error Responses

Errors return JSON with consistent structure:

```bash
$ vfs cat --json /nonexistent
{
  "error": "NotFound",
  "message": "File not found: /nonexistent",
  "path": "/nonexistent"
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
$ vfs snapshot save before-experiment

# Agent does work...
$ vfs mkdir /experiment
$ vfs write /experiment/code.py "print('test')"
$ vfs rm /important/file.txt  # Oops!

# Restore to saved state
$ vfs snapshot restore before-experiment

# /important/file.txt is back, /experiment is gone
```

### Snapshot Commands

```bash
# List all snapshots
vfs snapshot list

# Save with description
vfs snapshot save checkpoint-1 -d "Before refactoring"

# Get snapshot info
vfs snapshot info checkpoint-1

# Delete a snapshot
vfs snapshot delete checkpoint-1
```

### Agent Pattern: Experiment Loop

```python
def experiment_safely(vfs, experiment_fn):
    """Run experiment with automatic rollback on failure."""
    # Save state
    snapshot = vfs.run("snapshot", "save")
    snapshot_name = snapshot["name"]

    try:
        result = experiment_fn()
        return {"success": True, "result": result}
    except Exception as e:
        # Rollback on failure
        vfs.run("snapshot", "restore", snapshot_name)
        return {"success": False, "error": str(e), "rolled_back": True}
    finally:
        # Clean up snapshot
        vfs.run("snapshot", "delete", snapshot_name)
```

## Quotas

Quotas prevent agents from consuming excessive resources.

### Setting Quotas

```bash
# Set limits
vfs quota set max_size_mb 100       # Total vault size
vfs quota set max_files 10000       # Maximum file count
vfs quota set max_file_size_mb 10   # Single file limit

# View current limits
vfs quota
```

### Quota Errors

When quotas are exceeded, operations fail with clear errors:

```json
{
  "error": "QuotaExceeded",
  "message": "File size (15 MB) exceeds max_file_size_mb (10 MB)",
  "type": "max_file_size_mb",
  "requested": 15,
  "limit": 10
}
```

### Agent Pattern: Check Before Write

```python
def safe_write(vfs, path, content):
    """Write with quota awareness."""
    info = vfs.run("stats")

    content_size_mb = len(content.encode()) / (1024 * 1024)

    # Check against limits before writing
    if content_size_mb > 10:  # max_file_size_mb
        raise Exception(f"Content too large: {content_size_mb:.1f}MB")

    return vfs.run("write", path, content)
```

## Audit Log

The audit log records all operations for debugging and analysis.

### Viewing the Log

```bash
# Recent operations
$ vfs audit --limit 10

# JSON output
$ vfs audit --json --limit 100

# Filter by time
$ vfs audit --since "2024-03-10T14:00:00Z"
```

### Audit Log Fields

| Field | Description |
|-------|-------------|
| `timestamp` | ISO 8601 timestamp |
| `operation` | Command name (write, rm, mkdir, etc.) |
| `path` | Primary path involved |
| `result` | "ok" or error type |

### Clearing the Log

```bash
vfs audit clear --before "2024-01-01"
```

## Complete Agent Example

```python
#!/usr/bin/env python3
"""Example agent harness using vfs."""

import subprocess
import json

class VFSAgent:
    def __init__(self, vault_name="agent-workspace"):
        self.vault = vault_name

    def run(self, *args):
        """Execute vfs command, return parsed JSON."""
        cmd = ["vfs", "--vault", self.vault, "--json"] + [str(a) for a in args]
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

# Usage
agent = VFSAgent("demo-agent")
agent.mkdir("/project/src")
agent.write("/project/src/main.py", "print('Hello!')")
```

## Best Practices

### 1. Always Use JSON Mode

```python
# Good - parseable
result = subprocess.run(["vfs", "--json", "ls", "/"], capture_output=True)
data = json.loads(result.stdout)

# Bad - fragile text parsing
result = subprocess.run(["vfs", "ls", "/"], capture_output=True)
```

### 2. Checkpoint Before Risky Operations

```python
def risky_operation(agent):
    cp = agent.checkpoint()
    try:
        # Do risky stuff
        pass
    except:
        agent.rollback(cp["name"])
        raise
```

### 3. Set Appropriate Quotas

```bash
# For code generation agents
vfs quota set max_size_mb 50
vfs quota set max_files 500
vfs quota set max_file_size_mb 1

# For data processing agents
vfs quota set max_size_mb 1000
vfs quota set max_files 10000
```

### 4. Use Audit Logs for Debugging

```python
def diagnose_issue(agent):
    audit = agent.run("audit", "--limit", "100")
    errors = [e for e in audit["entries"] if e["result"] != "ok"]
    print(f"Recent errors: {len(errors)}")
```

### 5. Clean Up Snapshots

```python
def cleanup_old_snapshots(agent, keep=5):
    snapshots = agent.run("snapshot", "list")["snapshots"]
    snapshots.sort(key=lambda s: s["created"])
    for snap in snapshots[:-keep]:
        agent.run("snapshot", "delete", snap["name"])
```
