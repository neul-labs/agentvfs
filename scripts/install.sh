#!/bin/bash
# install.sh - Install avfs (agentvfs) binary
#
# Usage:
#   curl -sSfL https://raw.githubusercontent.com/neul-labs/agentvfs/main/scripts/install.sh | bash
#   ./scripts/install.sh [OPTIONS]
#
# Options:
#   --version VERSION   Install specific version (default: latest)
#   --prefix PATH       Install to PATH/bin (default: ~/.local or /usr/local)
#   --local             Install from local build instead of downloading
#   --force             Force reinstall even if already installed
#   --verbose           Show detailed output
#   --help              Show this help message

set -e

# Configuration
REPO="neul-labs/agentvfs"
BINARY_NAME="avfs"
DEFAULT_VERSION="latest"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Options
VERSION="${DEFAULT_VERSION}"
PREFIX=""
LOCAL_INSTALL=false
FORCE=false
VERBOSE=false

# Print functions
info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

success() {
    echo -e "${GREEN}[OK]${NC} $1"
}

warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1" >&2
}

debug() {
    if [ "$VERBOSE" = true ]; then
        echo -e "${BLUE}[DEBUG]${NC} $1"
    fi
}

compute_sha256() {
    local file=$1
    if command -v sha256sum &> /dev/null; then
        sha256sum "$file" | awk '{print $1}'
    elif command -v shasum &> /dev/null; then
        shasum -a 256 "$file" | awk '{print $1}'
    else
        return 1
    fi
}

# Show help
show_help() {
    cat << EOF
Install avfs (agentvfs) - Virtual filesystem CLI for AI agents

Usage:
  curl -sSfL https://raw.githubusercontent.com/neul-labs/agentvfs/main/scripts/install.sh | bash
  ./scripts/install.sh [OPTIONS]

Options:
  --version VERSION   Install specific version (default: latest)
  --prefix PATH       Install to PATH/bin (default: ~/.local or /usr/local)
  --local             Install from local build instead of downloading
  --force             Force reinstall even if already installed
  --verbose           Show detailed output
  --help              Show this help message

Examples:
  # Install latest version
  curl -sSfL https://raw.githubusercontent.com/neul-labs/agentvfs/main/scripts/install.sh | bash

  # Install specific version
  ./scripts/install.sh --version 0.1.0

  # Install from local build
  ./scripts/install.sh --local

  # Install to custom location
  ./scripts/install.sh --prefix /opt/avfs

EOF
}

# Parse arguments
parse_args() {
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --version)
                VERSION="$2"
                shift 2
                ;;
            --prefix)
                PREFIX="$2"
                shift 2
                ;;
            --local)
                LOCAL_INSTALL=true
                shift
                ;;
            --force)
                FORCE=true
                shift
                ;;
            --verbose)
                VERBOSE=true
                shift
                ;;
            --help|-h)
                show_help
                exit 0
                ;;
            *)
                error "Unknown option: $1"
                show_help
                exit 1
                ;;
        esac
    done
}

# Detect OS
detect_os() {
    local os
    case "$(uname -s)" in
        Linux*)     os="linux" ;;
        Darwin*)    os="darwin" ;;
        MINGW*|MSYS*|CYGWIN*) os="windows" ;;
        *)          os="unknown" ;;
    esac
    echo "$os"
}

# Detect architecture
detect_arch() {
    local arch
    case "$(uname -m)" in
        x86_64|amd64)   arch="x86_64" ;;
        aarch64|arm64)  arch="aarch64" ;;
        armv7l)         arch="armv7" ;;
        *)              arch="unknown" ;;
    esac
    echo "$arch"
}

# Get install directory
get_install_dir() {
    if [ -n "$PREFIX" ]; then
        echo "$PREFIX/bin"
    elif [ -w "/usr/local/bin" ]; then
        echo "/usr/local/bin"
    else
        echo "$HOME/.local/bin"
    fi
}

# Get latest version from GitHub
get_latest_version() {
    local latest
    latest=$(curl -sSfL "https://api.github.com/repos/${REPO}/releases/latest" 2>/dev/null | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/' | sed 's/^v//')
    if [ -z "$latest" ]; then
        echo ""
    else
        echo "$latest"
    fi
}

# Download and install from GitHub releases
install_from_release() {
    local os="$1"
    local arch="$2"
    local version="$3"
    local install_dir="$4"

    # Determine file extension
    local ext="tar.gz"
    if [ "$os" = "windows" ]; then
        ext="zip"
    fi

    # Build download URL
    local filename="${BINARY_NAME}-${version}-${os}-${arch}.${ext}"
    local url="https://github.com/${REPO}/releases/download/v${version}/${filename}"

    debug "Download URL: $url"

    # Create temp directory
    local tmp_dir
    tmp_dir=$(mktemp -d)
    trap "rm -rf $tmp_dir" EXIT

    # Download
    info "Downloading ${BINARY_NAME} v${version} for ${os}-${arch}..."
    if ! curl -sSfL "$url" -o "${tmp_dir}/${filename}" 2>/dev/null; then
        return 1
    fi

    local checksums_url="https://github.com/${REPO}/releases/download/v${version}/SHA256SUMS"
    if ! curl -sSfL "$checksums_url" -o "${tmp_dir}/SHA256SUMS" 2>/dev/null; then
        error "Failed to download release checksums"
        return 1
    fi

    if ! command -v sha256sum &> /dev/null && ! command -v shasum &> /dev/null; then
        error "sha256sum or shasum is required to verify the download"
        return 1
    fi

    local expected actual
    expected=$(grep "  ${filename}\$" "${tmp_dir}/SHA256SUMS" | awk '{print $1}')
    if [ -z "$expected" ]; then
        error "Checksum for ${filename} not found in release manifest"
        return 1
    fi

    actual=$(compute_sha256 "${tmp_dir}/${filename}") || {
        error "Failed to compute checksum for ${filename}"
        return 1
    }

    if [ "$expected" != "$actual" ]; then
        error "Checksum verification failed for ${filename}"
        return 1
    fi

    # Extract
    info "Extracting..."
    cd "$tmp_dir"
    if [ "$ext" = "zip" ]; then
        unzip -q "$filename"
    else
        tar -xzf "$filename"
    fi

    # Find binary
    local binary_path
    if [ "$os" = "windows" ]; then
        binary_path=$(find . -name "${BINARY_NAME}.exe" -type f | head -1)
    else
        binary_path=$(find . -name "${BINARY_NAME}" -type f | head -1)
    fi

    if [ -z "$binary_path" ]; then
        error "Binary not found in archive"
        return 1
    fi

    # Install
    mkdir -p "$install_dir"
    cp "$binary_path" "$install_dir/"
    chmod +x "${install_dir}/${BINARY_NAME}"

    return 0
}

# Install using cargo
install_from_cargo() {
    info "Installing via cargo..."

    if ! command -v cargo &> /dev/null; then
        return 1
    fi

    if [ "$VERSION" = "latest" ]; then
        cargo install agentvfs
    else
        cargo install agentvfs --version "$VERSION"
    fi

    return 0
}

# Install from local build
install_from_local() {
    local install_dir="$1"

    # Check if we're in the repo directory
    if [ ! -f "Cargo.toml" ]; then
        error "Not in agentvfs repository directory"
        return 1
    fi

    # Check for existing release build
    local release_binary="./target/release/${BINARY_NAME}"
    local debug_binary="./target/debug/${BINARY_NAME}"

    if [ -f "$release_binary" ]; then
        info "Using existing release build..."
        mkdir -p "$install_dir"
        cp "$release_binary" "$install_dir/"
        chmod +x "${install_dir}/${BINARY_NAME}"
        return 0
    fi

    if [ -f "$debug_binary" ]; then
        warn "Using debug build (consider running 'cargo build --release' for better performance)"
        mkdir -p "$install_dir"
        cp "$debug_binary" "$install_dir/"
        chmod +x "${install_dir}/${BINARY_NAME}"
        return 0
    fi

    # Build from source
    if command -v cargo &> /dev/null; then
        info "Building from source..."
        cargo build --release
        mkdir -p "$install_dir"
        cp "$release_binary" "$install_dir/"
        chmod +x "${install_dir}/${BINARY_NAME}"
        return 0
    fi

    error "No binary found and cargo not available"
    return 1
}

# Check if already installed
check_existing() {
    local install_dir="$1"
    local binary_path="${install_dir}/${BINARY_NAME}"

    if [ -f "$binary_path" ] && [ "$FORCE" = false ]; then
        local current_version
        current_version=$("$binary_path" --version 2>/dev/null | awk '{print $2}' || echo "unknown")
        warn "${BINARY_NAME} is already installed (version: ${current_version})"
        warn "Use --force to reinstall"
        exit 0
    fi
}

# Add to PATH if needed
suggest_path() {
    local install_dir="$1"

    if [[ ":$PATH:" != *":${install_dir}:"* ]]; then
        echo ""
        warn "${install_dir} is not in your PATH"
        echo ""
        echo "Add this to your shell profile (~/.bashrc, ~/.zshrc, etc.):"
        echo ""
        echo "  export PATH=\"${install_dir}:\$PATH\""
        echo ""
    fi
}

# Main installation logic
main() {
    parse_args "$@"

    echo ""
    echo "=========================================="
    echo "  Installing ${BINARY_NAME} (agentvfs)"
    echo "=========================================="
    echo ""

    local os arch install_dir
    os=$(detect_os)
    arch=$(detect_arch)
    install_dir=$(get_install_dir)

    debug "OS: $os"
    debug "Architecture: $arch"
    debug "Install directory: $install_dir"
    debug "Version: $VERSION"
    debug "Local install: $LOCAL_INSTALL"

    # Check for existing installation
    check_existing "$install_dir"

    # Resolve version
    if [ "$VERSION" = "latest" ] && [ "$LOCAL_INSTALL" = false ]; then
        info "Fetching latest version..."
        VERSION=$(get_latest_version)
        if [ -z "$VERSION" ]; then
            warn "Could not determine latest version, will try cargo install"
        else
            info "Latest version: ${VERSION}"
        fi
    fi

    # Try installation methods in order
    local installed=false

    # Method 1: Local install
    if [ "$LOCAL_INSTALL" = true ]; then
        if install_from_local "$install_dir"; then
            installed=true
        else
            error "Local installation failed"
            exit 1
        fi
    fi

    # Method 2: GitHub release
    if [ "$installed" = false ] && [ -n "$VERSION" ] && [ "$VERSION" != "latest" ]; then
        if [ "$os" != "unknown" ] && [ "$arch" != "unknown" ]; then
            if install_from_release "$os" "$arch" "$VERSION" "$install_dir"; then
                installed=true
            else
                warn "Pre-built binary not available for ${os}-${arch}"
            fi
        fi
    fi

    # Method 3: Cargo install
    if [ "$installed" = false ]; then
        if install_from_cargo; then
            installed=true
            # cargo install puts binary in ~/.cargo/bin
            install_dir="$HOME/.cargo/bin"
        fi
    fi

    # Method 4: Local build (if in repo)
    if [ "$installed" = false ] && [ -f "Cargo.toml" ]; then
        warn "Falling back to local build..."
        if install_from_local "$install_dir"; then
            installed=true
        fi
    fi

    if [ "$installed" = false ]; then
        error "Installation failed. Please try:"
        echo "  1. Install Rust: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
        echo "  2. Run: cargo install agentvfs"
        exit 1
    fi

    # Verify installation
    local binary_path="${install_dir}/${BINARY_NAME}"
    if [ -f "$binary_path" ]; then
        local installed_version
        installed_version=$("$binary_path" --version 2>/dev/null | awk '{print $2}' || echo "unknown")

        echo ""
        success "Installation complete!"
        echo ""
        echo "  Binary: ${binary_path}"
        echo "  Version: ${installed_version}"
        echo ""

        suggest_path "$install_dir"

        echo "Get started:"
        echo ""
        echo "  ${BINARY_NAME} vault create my-project"
        echo "  ${BINARY_NAME} mkdir /docs"
        echo "  ${BINARY_NAME} write /docs/hello.txt 'Hello, World!'"
        echo "  ${BINARY_NAME} cat /docs/hello.txt"
        echo ""
    else
        success "Installation complete via cargo!"
        echo ""
        echo "Run '${BINARY_NAME} --help' to get started."
        echo ""
    fi
}

main "$@"
