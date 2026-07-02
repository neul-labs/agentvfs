# agentvfs-cli

**agentvfs is a workspace runtime and execution boundary for AI agents.** It gives autonomous agents an isolated, forkable workspace with a single proxy surface for running commands — checkpoints, rollback, timeouts, and change tracking included — instead of handing them raw shell access to your host.

This package is the Python wrapper for the AgentVFS (`avfs`) CLI. The PyPI distribution is `agentvfs-cli`; the importable module is still `agentvfs` (`from agentvfs import AVFS`).

- **Website:** https://agentvfs.neullabs.com
- **Documentation:** https://docs.neullabs.com/agentvfs
- **GitHub:** https://github.com/neul-labs/agentvfs

## Install

```bash
pip install agentvfs-cli
```

## Native binary resolution

Calling `agentvfs ...` (or using the Python `AVFS` client, which shells out to `avfs`) resolves the native binary in this order:

1. `AGENTVFS_BIN` environment variable (explicit override)
2. `avfs` already on `$PATH` (e.g. installed via `cargo install agentvfs`, the `install.sh` script, or `npm install -g agentvfs-cli`)
3. Cached binary at `$XDG_CACHE_HOME/agentvfs/<version>/avfs` (or `~/.cache/agentvfs/<version>/avfs`)
4. **Lazy first-run download** — if no binary is found, the matching platform archive is fetched from the GitHub release for this package's version, extracted into the cache, and then exec'd. Subsequent invocations reuse the cached binary instantly.

This means `pip install agentvfs-cli && agentvfs --help` works out of the box on any supported platform — no extra install step required.

### Cache management

The cache is keyed by package version, so `pip install -U agentvfs-cli` automatically triggers a fresh download for the new release. Older version directories sit alongside (a few MB each); clean them with:

```bash
rm -rf ~/.cache/agentvfs
```

To pin a specific binary path (e.g. a locally-built dev `avfs`):

```bash
export AGENTVFS_BIN=/path/to/avfs
agentvfs --version
```

### Supported platforms

Linux x86_64 / aarch64, macOS x86_64 / Apple Silicon, Windows x86_64. Requests on other platforms fail at resolve time with a clear error and a pointer to the GitHub releases page.

### Network-restricted environments

If the install host has no internet egress, install the `avfs` binary out of band (e.g. unpack a release tarball into `/usr/local/bin`) and the PATH lookup in step 2 will pick it up.

## Usage

```python
from agentvfs import AVFS

avfs = AVFS("myproject")
avfs.mkdir("/src")
avfs.write("/src/main.py", "print('hello')")
result = avfs.proxy_exec("python", "/src/main.py")
print(result["stdout"])
```

## API

- Vaults: `create_vault`, `use_vault`, `fork_vault`, `list_vaults`
- Filesystem: `mkdir`, `write`, `cat`, `ls`, `rm`, `cp`, `mv`, `tree`
- Proxy: `proxy_exec`
- Checkpoints: `checkpoint_save`, `checkpoint_restore`, `checkpoint_list`
- Search: `grep`, `search`
- Maintenance: `stats`, `gc`, `compact`

## Part of the Neul Labs toolchain

Part of the [Neul Labs](https://www.neullabs.com) agent-infrastructure toolchain:

| Project | Description |
| --- | --- |
| [memorg](https://memorg.neullabs.com) | Give your LLM a memory that actually works. |
| [ormai](https://ormai.neullabs.com) | Give your AI agents database access without the risk — safe text-to-SQL. |
| [mcp-pay](https://mcp-pay.neullabs.com) | Payment awareness layer for MCP (Model Context Protocol). |
| [closegate](https://closegate.neullabs.com) | The policy chokepoint for finance AI agents. |
| [regulus](https://regulus.neullabs.com) | The EU & UK compliance plane for Google ADK. |
