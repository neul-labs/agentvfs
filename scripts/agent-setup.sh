#!/bin/bash
# agent-setup.sh - Create, mount, and enter an avfs vault for AI agents
#
# Usage:
#   source scripts/agent-setup.sh [vault-name] [mount-point]
#
# Examples:
#   source scripts/agent-setup.sh                    # Uses defaults
#   source scripts/agent-setup.sh my-project         # Custom vault name
#   source scripts/agent-setup.sh my-project /mnt/vfs # Custom mount point
#
# This script must be sourced (not executed) for the cd to take effect.

set -e

VAULT_NAME="${1:-agent-workspace}"
MOUNT_POINT="${2:-/tmp/avfs-$VAULT_NAME}"

# Check if avfs is installed
if ! command -v avfs &> /dev/null; then
    echo "Error: avfs command not found. Please install agentvfs first."
    echo "  cargo install agentvfs"
    return 1 2>/dev/null || exit 1
fi

# Create vault if it doesn't exist
if ! avfs vault list 2>/dev/null | grep -q "^$VAULT_NAME$"; then
    echo "Creating vault '$VAULT_NAME'..."
    avfs vault create "$VAULT_NAME"
else
    echo "Using existing vault '$VAULT_NAME'"
fi

# Set as current vault
avfs vault use "$VAULT_NAME"

# Create mount point directory
mkdir -p "$MOUNT_POINT"

# Check if FUSE feature is available
if avfs mount --help &>/dev/null; then
    # Check if already mounted
    if mountpoint -q "$MOUNT_POINT" 2>/dev/null; then
        echo "Vault already mounted at '$MOUNT_POINT'"
    else
        echo "Mounting vault at '$MOUNT_POINT'..."
        avfs mount "$VAULT_NAME" "$MOUNT_POINT"
    fi

    # Change to mount point
    cd "$MOUNT_POINT"
    echo ""
    echo "=== Agent Workspace Ready ==="
    echo "Vault:       $VAULT_NAME"
    echo "Mount point: $MOUNT_POINT"
    echo "Working dir: $(pwd)"
    echo ""
    echo "Use standard filesystem commands (ls, cat, mkdir, etc.)"
    echo "Changes are persisted to the vault database."
    echo ""
    echo "To unmount: avfs unmount $MOUNT_POINT"
else
    echo ""
    echo "=== Agent Workspace Ready (CLI mode) ==="
    echo "Vault: $VAULT_NAME"
    echo ""
    echo "FUSE mounting not available. Use avfs CLI commands:"
    echo "  avfs ls /"
    echo "  avfs mkdir /workspace"
    echo "  avfs write /workspace/file.txt \"content\""
    echo "  avfs cat /workspace/file.txt"
    echo ""
    echo "To enable FUSE mounting, rebuild with:"
    echo "  cargo build --release --features fuse"
fi
