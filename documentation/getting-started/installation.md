# Installation

## Quick Install (Recommended)

The fastest way to install agentvfs:

```bash
curl -sSfL https://raw.githubusercontent.com/neul-labs/agentvfs/main/install.sh | bash
```

### Install with Features

```bash
# Install with optional features
curl -sSfL https://raw.githubusercontent.com/neul-labs/agentvfs/main/install.sh | bash -s -- --cargo --features "sled-backend,lmdb-backend"
```

This script will:

1. Download the latest pre-built binary for your platform
2. Fall back to `cargo install` if pre-built binary unavailable
3. Fall back to building from source if cargo install fails

## Pre-built Binaries

Download from [GitHub Releases](https://github.com/neul-labs/agentvfs/releases):

| Platform | Architecture | Download |
|----------|--------------|----------|
| Linux | x86_64 | `avfs-VERSION-linux-x86_64.tar.gz` |
| Linux | ARM64 | `avfs-VERSION-linux-aarch64.tar.gz` |
| macOS | x86_64 (Intel) | `avfs-VERSION-darwin-x86_64.tar.gz` |
| macOS | ARM64 (Apple Silicon) | `avfs-VERSION-darwin-aarch64.tar.gz` |
| Windows | x86_64 | `avfs-VERSION-windows-x86_64.zip` |

### Manual Installation

```bash
# Download (example for Linux x86_64)
curl -LO https://github.com/neul-labs/agentvfs/releases/latest/download/avfs-0.1.0-linux-x86_64.tar.gz

# Extract
tar -xzf avfs-0.1.0-linux-x86_64.tar.gz

# Move to PATH
sudo mv avfs /usr/local/bin/

# Verify
avfs --version
```

## From crates.io

```bash
cargo install agentvfs

# With optional backends
cargo install agentvfs --features "sled-backend,lmdb-backend"

# With FUSE support (Linux/macOS)
cargo install agentvfs --features fuse

# With all features
cargo install agentvfs --features "sled-backend,lmdb-backend,fuse"
```

### Available Features

| Feature | Description |
|---------|-------------|
| `sled-backend` | Sled storage backend with Tantivy search |
| `lmdb-backend` | LMDB storage backend with Tantivy search |
| `fuse` | FUSE filesystem mounting support |

## From Source

### Requirements

- **Rust** 1.70 or later
- **Operating System**: Linux, macOS, or Windows

### Build

```bash
# Clone the repository
git clone https://github.com/neul-labs/agentvfs
cd agentvfs

# Build release version
cargo build --release

# The binary will be at target/release/avfs
sudo cp target/release/avfs /usr/local/bin/
```

### With FUSE Support

FUSE allows mounting vaults as real directories. Requires FUSE libraries:

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

Then build with the fuse feature:

```bash
cargo build --release --features fuse
```

## Verify Installation

```bash
# Check version
avfs --version

# Show help
avfs --help

# Create your first vault
avfs vault create test
avfs vault list
```

Expected output:

```
* test    ~/.avfs/test.avfs    (current)
```

## Shell Completion

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

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `AVFS_HOME` | Base directory for avfs data | `~/.avfs` |
| `AVFS_DEFAULT_VAULT` | Default vault name | `default` |

## Troubleshooting

### "Command not found"

Ensure the binary is in your PATH:

```bash
# Check if avfs is accessible
which avfs

# If not found, add to PATH
export PATH="$PATH:/usr/local/bin"
```

### FUSE Mount Errors

If mounting fails with permission errors:

```bash
# Check if FUSE is available
ls -la /dev/fuse

# Add your user to the fuse group
sudo usermod -aG fuse $USER
# Then log out and back in
```

### Database Locked

If you get "database is locked" errors:

```bash
# Check for running avfs processes
ps aux | grep avfs

# View vault info
avfs vault info myvault
```

## Next Steps

- [Quick Start Guide](quickstart.md) - Learn the basics in 5 minutes
- [Core Concepts](concepts.md) - Understand vaults, versioning, and more
- [Shell Usage](../user-guide/shell.md) - Interactive shell features
