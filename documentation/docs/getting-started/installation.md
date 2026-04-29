# Installation

agentvfs ships through four official channels. Pick whichever fits your environment — they all give you the same `avfs` binary.

| Channel       | Command                              | Native binary handling                              | Best for                          |
|---------------|--------------------------------------|-----------------------------------------------------|-----------------------------------|
| Shell installer | `curl … install.sh \| bash`        | Downloads from GitHub Releases, falls back to cargo | Most users, CI                    |
| Cargo         | `cargo install agentvfs`             | Builds from source                                  | Rust devs, custom feature flags   |
| npm           | `npm install -g agentvfs-cli`        | Downloads from GitHub Releases (postinstall)        | Node toolchains, JS-first projects |
| pip           | `pip install agentvfs-cli`           | Lazy first-run download into `~/.cache/agentvfs/`   | Python toolchains, agent runtimes |

Whatever you pick, the resulting CLI is identical: `avfs --version`, `avfs vault create …`, etc.

## Shell installer (recommended)

The fastest path:

```bash
curl -sSfL https://raw.githubusercontent.com/neul-labs/agentvfs/main/install.sh | bash
```

The script tries pre-built binaries first, falls back to `cargo install` if no archive matches your platform, then falls back to building from source.

### With optional features

```bash
curl -sSfL https://raw.githubusercontent.com/neul-labs/agentvfs/main/install.sh | \
    bash -s -- --cargo --features "sled-backend,lmdb-backend"
```

## Cargo

```bash
cargo install agentvfs

# With optional backends
cargo install agentvfs --features "sled-backend,lmdb-backend"

# With FUSE support (Linux/macOS)
cargo install agentvfs --features fuse

# Everything
cargo install agentvfs --features "sled-backend,lmdb-backend,fuse"
```

### Available features

| Feature        | Description                                  |
|----------------|----------------------------------------------|
| `sled-backend` | Sled storage backend with Tantivy search     |
| `lmdb-backend` | LMDB storage backend with Tantivy search     |
| `fuse`         | FUSE filesystem mounting (Linux/macOS only)  |

## npm

```bash
npm install -g agentvfs-cli
```

The npm package is a thin wrapper. A `postinstall` script downloads the platform-specific `avfs` binary from GitHub Releases into the package's `vendor/` directory, and the `bin` entry execs it. Both `avfs` and `agentvfs` end up on `$PATH`.

If the postinstall download fails (e.g. corporate proxy blocking github.com), the wrapper falls back to `cargo install agentvfs`.

## pip

```bash
pip install agentvfs-cli
```

The PyPI distribution is `agentvfs-cli`; the importable Python module is still `agentvfs` (`from agentvfs import AVFS`).

The pip wrapper does **not** ship the native binary. Instead, on first invocation of `agentvfs ...` (or first call from the Python `AVFS` client), it resolves the binary in this order:

1. `AGENTVFS_BIN` environment variable (explicit override)
2. `avfs` already on `$PATH` (e.g. installed via cargo / shell installer / npm)
3. Cached binary at `~/.cache/agentvfs/<version>/avfs`
4. Lazy download from the matching GitHub release into the cache, then exec

This means `pip install agentvfs-cli && agentvfs --help` works on a clean machine with internet access — no extra install step. Subsequent invocations reuse the cached binary instantly.

To force a re-download (corrupted cache):

```bash
rm -rf ~/.cache/agentvfs
```

To pin a specific binary path (locally-built dev `avfs`):

```bash
export AGENTVFS_BIN=/path/to/avfs
agentvfs --version
```

## Pre-built binaries

Download from [GitHub Releases](https://github.com/neul-labs/agentvfs/releases):

| Platform | Architecture           | Archive                                       |
|----------|------------------------|-----------------------------------------------|
| Linux    | x86_64                 | `avfs-<VERSION>-linux-x86_64.tar.gz`          |
| Linux    | ARM64                  | `avfs-<VERSION>-linux-aarch64.tar.gz`         |
| macOS    | x86_64 (Intel)         | `avfs-<VERSION>-darwin-x86_64.tar.gz`         |
| macOS    | ARM64 (Apple Silicon)  | `avfs-<VERSION>-darwin-aarch64.tar.gz`        |
| Windows  | x86_64                 | `avfs-<VERSION>-windows-x86_64.zip`           |

Manual extraction:

```bash
curl -LO https://github.com/neul-labs/agentvfs/releases/latest/download/avfs-<VERSION>-linux-x86_64.tar.gz
tar -xzf avfs-<VERSION>-linux-x86_64.tar.gz
sudo mv avfs /usr/local/bin/
avfs --version
```

## From source

### Requirements

- Rust 1.70 or later
- Linux, macOS, or Windows

### Build

```bash
git clone https://github.com/neul-labs/agentvfs
cd agentvfs
cargo build --release
sudo cp target/release/avfs /usr/local/bin/
```

### With FUSE support

FUSE allows mounting vaults as real directories. Requires system FUSE libraries:

=== "Ubuntu/Debian"

    ```bash
    sudo apt-get install libfuse3-dev
    ```

=== "Fedora"

    ```bash
    sudo dnf install fuse3-devel
    ```

=== "macOS"

    ```bash
    brew install macfuse
    ```

Then build with the feature:

```bash
cargo build --release --features fuse
```

## Verify

```bash
avfs --version
avfs --help
avfs vault create test
avfs vault list
```

Expected:

```
* test    ~/.avfs/test.avfs    (current)
```

## Shell completion

Generate aliases for your shell:

=== "Bash"

    ```bash
    # Add to ~/.bashrc
    eval "$(avfs aliases --format bash)"
    ```

=== "Zsh"

    ```bash
    # Add to ~/.zshrc
    eval "$(avfs aliases --format zsh)"
    ```

=== "Fish"

    ```bash
    # Add to ~/.config/fish/config.fish
    avfs aliases --format fish | source
    ```

## Configuration

agentvfs stores data in `~/.avfs/` by default:

```
~/.avfs/
├── config.json      # Global configuration
├── history          # Shell command history
├── default.avfs     # Default vault database
└── myproject.avfs   # Additional vaults
```

### Environment variables

| Variable             | Description                  | Default   |
|----------------------|------------------------------|-----------|
| `AVFS_HOME`          | Base directory for avfs data | `~/.avfs` |
| `AVFS_DEFAULT_VAULT` | Default vault name           | `default` |
| `AGENTVFS_BIN`       | Override binary path used by the pip / npm wrappers | (auto) |

## Troubleshooting

### "Command not found"

Ensure the binary is on `$PATH`:

```bash
which avfs
export PATH="$PATH:/usr/local/bin"
```

### FUSE mount errors

```bash
ls -la /dev/fuse
sudo usermod -aG fuse $USER   # then log out and back in
```

### Database locked

```bash
ps aux | grep avfs
avfs vault info myvault
```

### pip wrapper can't reach GitHub

If your install host has no egress, install `avfs` out of band (e.g. unpack a release tarball into `/usr/local/bin`) — the pip wrapper's PATH lookup will pick it up and skip the download step. Or set `AGENTVFS_BIN=/path/to/avfs` to point at a custom location.

## Next steps

- [Quick Start](quickstart.md) — learn the basics in 5 minutes
- [Core Concepts](concepts.md) — vaults, forks, checkpoints, mounts, proxy
- [Shell Usage](../user-guide/shell.md) — interactive shell features
- [Python SDK](../advanced/python-sdk.md) — embed avfs from Python
