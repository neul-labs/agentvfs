# Command Reference

This reference focuses on the command surface that matters for the workspace runtime and proxy-boundary model.

## Core Workspace Commands

### Create and Select Vaults

```bash
avfs vault create <name>
avfs vault list
avfs vault use <name>
avfs vault info [name]
avfs vault delete <name> -y
```

### Fork a Workspace

```bash
avfs vault fork <source> <name> [--use]
```

Use forks for task-scoped workspaces rather than mutating the root vault directly.

## Rollback Commands

### Checkpoints

```bash
avfs checkpoint save [name]
avfs checkpoint list
avfs checkpoint restore <name>
avfs checkpoint delete <name>
avfs checkpoint info <name>
```

`checkpoint` is the preferred rollback-oriented surface. `snapshot` remains available as the underlying implementation surface.

### Snapshots

```bash
avfs snapshot save [name]
avfs snapshot list
avfs snapshot restore <name>
avfs snapshot delete <name>
avfs snapshot info <name>
```

## Filesystem Commands

```bash
avfs ls [path]
avfs cat <path>
avfs write <path> [content]
avfs mkdir <path>
avfs rm <path>
avfs cp <src> <dst>
avfs mv <src> <dst>
avfs tree [path]
avfs pwd
```

## Search and Inspection

```bash
avfs grep <pattern> [path]
avfs find <pattern>
avfs search <query>
avfs log <path>
avfs diff <a> <b>
avfs audit [--limit N]
avfs stats
```

## Metadata

```bash
avfs tag <path> <tag>
avfs untag <path> <tag>
avfs meta <path> [key] [value]
```

## Import / Export

```bash
avfs import <real-path> <vfs-path>
avfs export <vfs-path> <real-path>
avfs exec <vfs-path> -- <command> ...
```

## Runtime and FUSE Commands

### Mount

```bash
avfs mount <vault> <mountpoint> [--foreground] [--readonly]
avfs unmount <mountpoint>
```

### Proxy

```bash
avfs proxy exec [--cwd /path] [--mountpoint /tmp/mount] [--readonly] -- <command> ...
avfs proxy exec --shell "npm test && npm run lint"
```

`proxy exec` is the current agent-facing execution boundary. It accepts one top-level command, applies policy, prepares the mounted workspace, runs the command, and reports the result.

With `--json`, `proxy exec` returns a versioned execution envelope that includes:

- request metadata
- policy decision
- stdout and stderr
- exit code and duration
- checkpoint created
- changed-files summary

## Shell Commands

```bash
avfs shell
avfs aliases
```

The interactive shell is useful for humans and manual exploration, but the long-term agent model should prefer proxy-mediated top-level command execution.

## Recommended Agent Pattern

The preferred workspace pattern is:

1. `vault create`
2. `vault fork`
3. `checkpoint save`
4. `mount` or `proxy`
5. inspect with `audit`, `log`, and `diff`

## Related Documents

- [Quick Start](../getting-started/quickstart.md)
- [Vault Management](../user-guide/vaults.md)
- [Proxy Boundary](../advanced/proxy-boundary.md)
- [Architecture](architecture.md)
