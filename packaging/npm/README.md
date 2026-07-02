# agentvfs-cli

**agentvfs is a workspace runtime and execution boundary for AI agents.** It gives autonomous agents an isolated, forkable workspace with a single proxy surface for running commands — checkpoints, rollback, timeouts, and change tracking included — instead of handing them raw shell access to your host.

This package is the cross-platform npm wrapper for the `avfs` CLI. It downloads the correct platform-specific native binary from GitHub releases during installation.

- **Website:** https://agentvfs.neullabs.com
- **Documentation:** https://docs.neullabs.com/agentvfs
- **GitHub:** https://github.com/neul-labs/agentvfs

## Install

```bash
npm install -g agentvfs-cli
```

## Usage

```bash
# Create an isolated workspace vault
avfs vault create myproject

# Checkpoint before risky work, then run a command through the proxy boundary
avfs checkpoint save before-refactor
avfs proxy exec -- python script.py
```

See the [full documentation](https://docs.neullabs.com/agentvfs) for vaults, forks, checkpoints, the proxy boundary, and agent-integration patterns.

## Supported Platforms

- Linux x86_64 / ARM64
- macOS x86_64 / Apple Silicon
- Windows x86_64

## Fallback

If the GitHub release download fails, the installer falls back to `cargo install agentvfs`.

## Part of the Neul Labs toolchain

Part of the [Neul Labs](https://www.neullabs.com) agent-infrastructure toolchain:

| Project | Description |
| --- | --- |
| [memorg](https://memorg.neullabs.com) | Give your LLM a memory that actually works. |
| [ormai](https://ormai.neullabs.com) | Give your AI agents database access without the risk — safe text-to-SQL. |
| [mcp-pay](https://mcp-pay.neullabs.com) | Payment awareness layer for MCP (Model Context Protocol). |
| [closegate](https://closegate.neullabs.com) | The policy chokepoint for finance AI agents. |
| [regulus](https://regulus.neullabs.com) | The EU & UK compliance plane for Google ADK. |
