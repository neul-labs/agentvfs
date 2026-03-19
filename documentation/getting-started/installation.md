# Installation

## Requirements

- **Rust** 1.70 or later (for building from source)
- **Operating System**: Linux, macOS, or Windows

### Optional Dependencies

For FUSE mount support (Linux/macOS only):

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

## Installation Methods

### From Source (Recommended)

```bash
# Clone the repository
git clone https://github.com/yourusername/avfs
cd avfs

# Build release version
cargo build --release

# The binary will be at target/release/avfs
# Optionally, copy to your PATH:
sudo cp target/release/avfs /usr/local/bin/
```

### With FUSE Support

To enable FUSE mounting capabilities:

```bash
# Build with FUSE feature
cargo build --release --features fuse
```

### From Crates.io

Once published:

```bash
cargo install avfs

# With FUSE support
cargo install avfs --features fuse
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

VFS can generate shell completion scripts:

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

VFS stores its data in `~/.avfs/` by default:

```
~/.avfs/
├── config.json     # Global configuration
├── default.avfs     # Default vault database
└── other.avfs       # Additional vaults
```

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `VFS_HOME` | Base directory for VFS data | `~/.avfs` |
| `VFS_DEFAULT_VAULT` | Default vault name | `default` |

## Troubleshooting

### "Command not found"

Ensure the binary is in your PATH:

```bash
# Check if avfs is accessible
which avfs

# If not found, add to PATH
export PATH="$PATH:/path/to/avfs/target/release"
```

### FUSE Mount Errors

If mounting fails with permission errors:

```bash
# Check if FUSE is available
ls -la /dev/fuse

# You may need to add your user to the fuse group
sudo usermod -aG fuse $USER
# Then log out and back in
```

### Database Locked

If you get "database is locked" errors, ensure no other VFS process is accessing the same vault:

```bash
# List running avfs processes
ps aux | grep avfs

# Force unlock (use with caution)
avfs vault info myvault
```

## Next Steps

- [Quick Start Guide](quickstart.md) - Learn the basics in 5 minutes
- [Core Concepts](concepts.md) - Understand how VFS works
