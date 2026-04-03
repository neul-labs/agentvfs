# FUSE Mount

FUSE mount support exposes a vault or fork as a real directory so normal CLI tools can operate on it.

In the long-term architecture, mounts are not the primary user-facing abstraction. They are the filesystem substrate behind the proxy boundary.

```text
agent -> proxy boundary -> mounted forked workspace -> cli tools
```

## Role in the System

Mounts matter because most developer tools expect:

- a real current working directory
- ordinary file paths
- normal file APIs

That makes FUSE the bridge between agentvfs storage and standard tools such as editors, build systems, and test runners.

## Requirements

FUSE support requires building with the `fuse` feature and having a system FUSE library installed.

### Linux

```bash
sudo apt-get install libfuse3-dev
```

### macOS

```bash
brew install macfuse
```

### Build

```bash
cargo build --release --features fuse
```

## Mount a Workspace

```bash
mkdir -p /tmp/avfs-mount
avfs mount myproject /tmp/avfs-mount --foreground
```

Read-only mount:

```bash
avfs mount myproject /tmp/avfs-mount --foreground --readonly
```

Unmount:

```bash
avfs unmount /tmp/avfs-mount
```

## Why This Matters for the Proxy Boundary

The proxy boundary should not make the agent manually reason about mount lifecycle. It should:

1. select a vault or fork
2. mount it
3. execute the top-level command
4. summarize the result
5. unmount or keep the session alive

So while `mount` and `unmount` remain useful low-level tools, they should mostly be runtime mechanics behind proxy execution.

## Typical Uses

Use a mount when you need a tool that expects a real filesystem:

- editors
- language servers
- compilers
- test runners
- shell scripts

Example:

```bash
avfs mount myproject-task-1 /tmp/avfs-mount --foreground
cd /tmp/avfs-mount
pytest
```

## Limitations

- no symlinks
- no hard links
- no extended attributes
- no deep execution policy by itself

The mount layer exposes a workspace. It does not decide whether the top-level command should have been allowed in the first place. That decision belongs to the proxy boundary.

## Related Guides

- [Proxy Boundary](proxy-boundary.md)
- [Agent Integration](agent-integration.md)
- [Architecture](../reference/architecture.md)
