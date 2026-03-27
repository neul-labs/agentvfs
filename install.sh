#!/bin/bash
# AgentVFS (avfs) Installation Script
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/neul-labs/agentvfs/main/install.sh | bash
#
# Options:
#   --version VERSION    Install specific version (default: latest)
#   --prefix PATH        Install to PATH/bin (default: ~/.local)
#   --force              Force reinstall even if already installed
#   --features FEATURES  Enable cargo features (e.g., sled-backend,lmdb-backend)
#   --quiet              Suppress non-error output
#   --help               Show this help message

set -e

# Configuration
REPO="neul-labs/agentvfs"
BINARY_NAME="avfs"
DEFAULT_PREFIX="$HOME/.local"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Print colored output
info() { echo -e "${BLUE}[INFO]${NC} $1"; }
success() { echo -e "${GREEN}[OK]${NC} $1"; }
warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
error() { echo -e "${RED}[ERROR]${NC} $1"; exit 1; }

# Show help
show_help() {
    cat << EOF
AgentVFS (avfs) Installation Script

USAGE:
    install.sh [OPTIONS]

OPTIONS:
    --version VERSION    Install specific version (default: latest)
    --prefix PATH        Install to PATH/bin (default: ~/.local)
    --force              Force reinstall even if already installed
    --cargo              Install using cargo instead of pre-built binary
    --features FEATURES  Enable cargo features (comma-separated)
    --quiet, -q          Suppress non-error output
    --help               Show this help message

AVAILABLE FEATURES:
    sled-backend         Enable Sled storage backend
    lmdb-backend         Enable LMDB storage backend
    fuse                 Enable FUSE filesystem mounting

EXAMPLES:
    # Install latest version
    ./install.sh

    # Install specific version
    ./install.sh --version 0.1.0

    # Install to /usr/local
    sudo ./install.sh --prefix /usr/local

    # Install using cargo with features
    ./install.sh --cargo --features "sled-backend,lmdb-backend"

    # Quiet install for CI
    ./install.sh --quiet
EOF
    exit 0
}

# Parse command line arguments
VERSION=""
PREFIX="$DEFAULT_PREFIX"
FORCE=false
USE_CARGO=false
FEATURES=""
QUIET=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --version)
            VERSION="$2"
            shift 2
            ;;
        --prefix)
            PREFIX="$2"
            shift 2
            ;;
        --force)
            FORCE=true
            shift
            ;;
        --cargo)
            USE_CARGO=true
            shift
            ;;
        --features)
            FEATURES="$2"
            shift 2
            ;;
        --quiet|-q)
            QUIET=true
            shift
            ;;
        --help|-h)
            show_help
            ;;
        *)
            error "Unknown option: $1"
            ;;
    esac
done

# Override output functions for quiet mode
if [[ "$QUIET" == true ]]; then
    info() { :; }
    success() { :; }
    warn() { :; }
fi

# Detect OS and architecture
detect_platform() {
    local os arch

    case "$(uname -s)" in
        Linux*)  os="linux" ;;
        Darwin*) os="darwin" ;;
        MINGW*|MSYS*|CYGWIN*) os="windows" ;;
        *)       error "Unsupported operating system: $(uname -s)" ;;
    esac

    case "$(uname -m)" in
        x86_64|amd64)  arch="x86_64" ;;
        aarch64|arm64) arch="aarch64" ;;
        armv7*)        arch="armv7" ;;
        *)             error "Unsupported architecture: $(uname -m)" ;;
    esac

    echo "${os}-${arch}"
}

# Get latest version from GitHub
get_latest_version() {
    local latest
    latest=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | sed -E 's/.*"v?([^"]+)".*/\1/')

    if [[ -z "$latest" ]]; then
        error "Failed to fetch latest version from GitHub"
    fi

    echo "$latest"
}

# Download and verify binary
download_binary() {
    local version="$1"
    local platform="$2"
    local install_dir="$3"

    local base_url="https://github.com/${REPO}/releases/download/v${version}"
    local archive_name="${BINARY_NAME}-${version}-${platform}"

    # Determine archive extension
    local ext="tar.gz"
    if [[ "$platform" == *"windows"* ]]; then
        ext="zip"
    fi

    local archive_url="${base_url}/${archive_name}.${ext}"
    local checksum_url="${base_url}/checksums.txt"

    info "Downloading ${BINARY_NAME} v${version} for ${platform}..."

    # Create temp directory
    local tmp_dir
    tmp_dir=$(mktemp -d)
    trap "rm -rf $tmp_dir" EXIT

    # Download archive
    if ! curl -fsSL "$archive_url" -o "$tmp_dir/archive.${ext}"; then
        warn "Pre-built binary not available for ${platform}"
        return 1
    fi

    # Download and verify checksum
    if curl -fsSL "$checksum_url" -o "$tmp_dir/checksums.txt" 2>/dev/null; then
        info "Verifying checksum..."
        local expected_checksum
        expected_checksum=$(grep "${archive_name}.${ext}" "$tmp_dir/checksums.txt" | awk '{print $1}')

        if [[ -n "$expected_checksum" ]]; then
            local actual_checksum
            if command -v sha256sum &> /dev/null; then
                actual_checksum=$(sha256sum "$tmp_dir/archive.${ext}" | awk '{print $1}')
            elif command -v shasum &> /dev/null; then
                actual_checksum=$(shasum -a 256 "$tmp_dir/archive.${ext}" | awk '{print $1}')
            else
                warn "No checksum tool available, skipping verification"
            fi

            if [[ -n "$actual_checksum" && "$expected_checksum" != "$actual_checksum" ]]; then
                error "Checksum verification failed!"
            fi
            success "Checksum verified"
        fi
    fi

    # Extract archive
    info "Extracting..."
    mkdir -p "$tmp_dir/extracted"

    if [[ "$ext" == "zip" ]]; then
        unzip -q "$tmp_dir/archive.${ext}" -d "$tmp_dir/extracted"
    else
        tar -xzf "$tmp_dir/archive.${ext}" -C "$tmp_dir/extracted"
    fi

    # Find and install binary
    local binary_path
    binary_path=$(find "$tmp_dir/extracted" -name "$BINARY_NAME" -o -name "${BINARY_NAME}.exe" | head -1)

    if [[ -z "$binary_path" ]]; then
        error "Binary not found in archive"
    fi

    mkdir -p "$install_dir"
    cp "$binary_path" "$install_dir/"
    chmod +x "$install_dir/${BINARY_NAME}"

    return 0
}

# Install using cargo
install_with_cargo() {
    local version="$1"

    if ! command -v cargo &> /dev/null; then
        error "Cargo not found. Please install Rust first: https://rustup.rs"
    fi

    info "Installing with cargo..."

    local cargo_args="agentvfs"
    if [[ -n "$version" ]]; then
        cargo_args="$cargo_args --version $version"
    fi
    if [[ -n "$FEATURES" ]]; then
        cargo_args="$cargo_args --features $FEATURES"
    fi

    cargo install $cargo_args
}

# Check if avfs is already installed
check_existing() {
    if command -v "$BINARY_NAME" &> /dev/null; then
        local current_version
        current_version=$("$BINARY_NAME" --version 2>/dev/null | awk '{print $2}' || echo "unknown")

        if [[ "$FORCE" != true ]]; then
            warn "${BINARY_NAME} is already installed (version ${current_version})"
            echo "Use --force to reinstall"
            exit 0
        fi
    fi
}

# Add to PATH if needed
setup_path() {
    local bin_dir="$1"

    # Check if already in PATH
    if echo "$PATH" | grep -q "$bin_dir"; then
        return 0
    fi

    warn "${bin_dir} is not in your PATH"

    # Detect shell and suggest adding to PATH
    local shell_name
    shell_name=$(basename "$SHELL")
    local rc_file=""

    case "$shell_name" in
        bash)
            if [[ -f "$HOME/.bashrc" ]]; then
                rc_file="$HOME/.bashrc"
            elif [[ -f "$HOME/.bash_profile" ]]; then
                rc_file="$HOME/.bash_profile"
            fi
            ;;
        zsh)
            rc_file="$HOME/.zshrc"
            ;;
        fish)
            rc_file="$HOME/.config/fish/config.fish"
            ;;
    esac

    if [[ -n "$rc_file" ]]; then
        echo ""
        echo "Add this to your $rc_file:"
        if [[ "$shell_name" == "fish" ]]; then
            echo "  set -gx PATH \"$bin_dir\" \$PATH"
        else
            echo "  export PATH=\"$bin_dir:\$PATH\""
        fi
        echo ""
        echo "Then restart your shell or run:"
        echo "  source $rc_file"
    fi
}

# Main installation
main() {
    echo ""
    echo "  AgentVFS Installer"
    echo "  =================="
    echo ""

    # Check for existing installation
    check_existing

    # Get version to install
    if [[ -z "$VERSION" ]]; then
        info "Fetching latest version..."
        VERSION=$(get_latest_version)
    fi

    info "Installing version: ${VERSION}"

    local install_dir="${PREFIX}/bin"

    if [[ "$USE_CARGO" == true ]]; then
        install_with_cargo "$VERSION"
    else
        # Try pre-built binary first
        local platform
        platform=$(detect_platform)
        info "Detected platform: ${platform}"

        if ! download_binary "$VERSION" "$platform" "$install_dir"; then
            info "Falling back to cargo install..."
            install_with_cargo "$VERSION"
        fi
    fi

    # Verify installation
    if [[ -f "${install_dir}/${BINARY_NAME}" ]]; then
        success "Installed to ${install_dir}/${BINARY_NAME}"
        setup_path "$install_dir"
    elif command -v "$BINARY_NAME" &> /dev/null; then
        success "Installed successfully!"
    else
        error "Installation failed"
    fi

    # Show version
    echo ""
    if command -v "$BINARY_NAME" &> /dev/null || [[ -f "${install_dir}/${BINARY_NAME}" ]]; then
        info "Verifying installation..."
        "${install_dir}/${BINARY_NAME}" --version 2>/dev/null || "$BINARY_NAME" --version
    fi

    echo ""
    success "Installation complete!"
    echo ""
    echo "Get started:"
    echo "  ${BINARY_NAME} vault create my-vault"
    echo "  ${BINARY_NAME} write /hello.txt \"Hello, World!\""
    echo "  ${BINARY_NAME} cat /hello.txt"
    echo ""
}

main
