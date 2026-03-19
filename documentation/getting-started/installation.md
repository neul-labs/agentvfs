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
git clone https://github.com/yourusername/vfs
cd vfs

# Build release version
cargo build --release

# The binary will be at target/release/vfs
# Optionally, copy to your PATH:
sudo cp target/release/vfs /usr/local/bin/
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
cargo install vfs

# With FUSE support
cargo install vfs --features fuse
```

## Verify Installation

```bash
# Check version
vfs --version

# Show help
vfs --help

# Create your first vault
vfs vault create test
vfs vault list
```

Expected output:

```
* test    ~/.vfs/test.vfs    (current)
```

## Shell Completion

VFS can generate shell completion scripts:

=== "Bash"

    ```bash
    # Add to ~/.bashrc
    eval "$(vfs aliases --format bash)"
    ```

=== "Zsh"

    ```bash
    # Add to ~/.zshrc
    eval "$(vfs aliases --format zsh)"
    ```

=== "Fish"

    ```bash
    # Add to ~/.config/fish/config.fish
    vfs aliases --format fish | source
    ```

## Configuration

VFS stores its data in `~/.vfs/` by default:

```
~/.vfs/
├── config.json     # Global configuration
├── default.vfs     # Default vault database
└── other.vfs       # Additional vaults
```

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `VFS_HOME` | Base directory for VFS data | `~/.vfs` |
| `VFS_DEFAULT_VAULT` | Default vault name | `default` |

## Troubleshooting

### "Command not found"

Ensure the binary is in your PATH:

```bash
# Check if vfs is accessible
which vfs

# If not found, add to PATH
export PATH="$PATH:/path/to/vfs/target/release"
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
# List running vfs processes
ps aux | grep vfs

# Force unlock (use with caution)
vfs vault info myvault
```

## Next Steps

- [Quick Start Guide](quickstart.md) - Learn the basics in 5 minutes
- [Core Concepts](concepts.md) - Understand how VFS works
