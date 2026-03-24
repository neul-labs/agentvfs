#!/bin/bash
#
# release.sh - Build and release agentvfs for multiple platforms
#
# Usage:
#   ./scripts/release.sh --version 0.1.0
#   ./scripts/release.sh --version 0.1.0 --dry-run
#   ./scripts/release.sh --version 0.1.0 --targets linux-x86_64,darwin-aarch64
#
# Requirements:
#   - cross (cargo install cross)
#   - gh CLI (for GitHub releases)
#   - tar, zip

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default values
VERSION=""
DRY_RUN=false
SKIP_BUILD=false
TARGETS=""
REPO="neul-labs/agentvfs"
BINARY_NAME="avfs"

# All supported targets
ALL_TARGETS=(
    "x86_64-unknown-linux-gnu:linux-x86_64:tar.gz"
    "aarch64-unknown-linux-gnu:linux-aarch64:tar.gz"
    "x86_64-apple-darwin:darwin-x86_64:tar.gz"
    "aarch64-apple-darwin:darwin-aarch64:tar.gz"
    "x86_64-pc-windows-gnu:windows-x86_64:zip"
)

# Print colored message
print_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Show usage
usage() {
    cat << EOF
Usage: $(basename "$0") [OPTIONS]

Build and release agentvfs for multiple platforms.

OPTIONS:
    --version VERSION    Version to release (required)
    --dry-run            Build but don't upload to GitHub
    --skip-build         Skip building, just create release from existing artifacts
    --targets TARGETS    Comma-separated list of targets (default: all)
                         Available: linux-x86_64, linux-aarch64, darwin-x86_64,
                                    darwin-aarch64, windows-x86_64
    -h, --help           Show this help message

EXAMPLES:
    # Build and release version 0.1.0
    $(basename "$0") --version 0.1.0

    # Dry run (build only, no GitHub release)
    $(basename "$0") --version 0.1.0 --dry-run

    # Build only Linux targets
    $(basename "$0") --version 0.1.0 --targets linux-x86_64,linux-aarch64

REQUIREMENTS:
    - cross: cargo install cross
    - gh: GitHub CLI for creating releases
    - tar, zip: For packaging

EOF
}

# Parse arguments
parse_args() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            --version)
                VERSION="$2"
                shift 2
                ;;
            --dry-run)
                DRY_RUN=true
                shift
                ;;
            --skip-build)
                SKIP_BUILD=true
                shift
                ;;
            --targets)
                TARGETS="$2"
                shift 2
                ;;
            -h|--help)
                usage
                exit 0
                ;;
            *)
                print_error "Unknown option: $1"
                usage
                exit 1
                ;;
        esac
    done

    if [[ -z "$VERSION" ]]; then
        print_error "Version is required. Use --version VERSION"
        usage
        exit 1
    fi
}

# Verify Cargo.toml version matches
verify_version() {
    local cargo_version
    cargo_version=$(grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')

    if [[ "$cargo_version" != "$VERSION" ]]; then
        print_error "Version mismatch: Cargo.toml has '$cargo_version', but you specified '$VERSION'"
        print_info "Update Cargo.toml or use --version $cargo_version"
        exit 1
    fi

    print_success "Version $VERSION matches Cargo.toml"
}

# Check required tools
check_requirements() {
    local missing=()

    if ! command -v cross &> /dev/null && [[ "$SKIP_BUILD" == "false" ]]; then
        missing+=("cross (cargo install cross)")
    fi

    if ! command -v gh &> /dev/null && [[ "$DRY_RUN" == "false" ]]; then
        missing+=("gh (GitHub CLI)")
    fi

    if ! command -v tar &> /dev/null; then
        missing+=("tar")
    fi

    if ! command -v zip &> /dev/null; then
        missing+=("zip")
    fi

    if ! command -v sha256sum &> /dev/null && ! command -v shasum &> /dev/null; then
        missing+=("sha256sum or shasum")
    fi

    if [[ ${#missing[@]} -gt 0 ]]; then
        print_error "Missing required tools:"
        for tool in "${missing[@]}"; do
            echo "  - $tool"
        done
        exit 1
    fi

    print_success "All required tools available"
}

compute_sha256() {
    local file=$1
    if command -v sha256sum &> /dev/null; then
        sha256sum "$file" | awk '{print $1}'
    else
        shasum -a 256 "$file" | awk '{print $1}'
    fi
}

# Get target info by short name
get_target_info() {
    local short_name=$1
    for target in "${ALL_TARGETS[@]}"; do
        IFS=':' read -r rust_target name ext <<< "$target"
        if [[ "$name" == "$short_name" ]]; then
            echo "$rust_target:$name:$ext"
            return 0
        fi
    done
    return 1
}

# Build for a specific target
build_target() {
    local rust_target=$1
    local name=$2

    print_info "Building for $name ($rust_target)..."

    # Use cross for cross-compilation
    if cross build --release --target "$rust_target"; then
        print_success "Built for $name"
        return 0
    else
        print_error "Failed to build for $name"
        return 1
    fi
}

# Package binary for a target
package_target() {
    local rust_target=$1
    local name=$2
    local ext=$3
    local artifact_dir="target/release-artifacts"
    local binary_path="target/${rust_target}/release/${BINARY_NAME}"
    local artifact_name="${BINARY_NAME}-${VERSION}-${name}"

    # Windows binary has .exe extension
    if [[ "$rust_target" == *"windows"* ]]; then
        binary_path="${binary_path}.exe"
    fi

    if [[ ! -f "$binary_path" ]]; then
        print_error "Binary not found: $binary_path"
        return 1
    fi

    mkdir -p "$artifact_dir"

    print_info "Packaging $artifact_name.$ext..." >&2

    # Create temp directory for packaging
    local pkg_dir
    pkg_dir=$(mktemp -d)

    if [[ "$rust_target" == *"windows"* ]]; then
        cp "$binary_path" "$pkg_dir/${BINARY_NAME}.exe"
    else
        cp "$binary_path" "$pkg_dir/${BINARY_NAME}"
        chmod +x "$pkg_dir/${BINARY_NAME}"
    fi

    # Copy README and LICENSE
    cp README.md "$pkg_dir/" 2>/dev/null || true
    cp LICENSE "$pkg_dir/" 2>/dev/null || true

    # Package based on extension
    local artifact_path="$artifact_dir/${artifact_name}.${ext}"

    if [[ "$ext" == "tar.gz" ]]; then
        tar -czf "$artifact_path" -C "$pkg_dir" .
    elif [[ "$ext" == "zip" ]]; then
        (cd "$pkg_dir" && zip -r "../${artifact_name}.${ext}" .)
        mv "$pkg_dir/../${artifact_name}.${ext}" "$artifact_path"
    fi

    rm -rf "$pkg_dir"

    print_success "Created $artifact_path" >&2
    echo "$artifact_path"
}

generate_checksums() {
    local artifact_dir="target/release-artifacts"
    local checksums_path="${artifact_dir}/SHA256SUMS"
    shift 0

    : > "$checksums_path"
    for artifact in "$@"; do
        local checksum
        checksum=$(compute_sha256 "$artifact")
        printf "%s  %s\n" "$checksum" "$(basename "$artifact")" >> "$checksums_path"
    done

    print_success "Created $checksums_path" >&2
    echo "$checksums_path"
}

# Create GitHub release
create_release() {
    local artifacts=("$@")
    local tag="v${VERSION}"

    print_info "Creating GitHub release $tag..."

    # Check if release already exists
    if gh release view "$tag" --repo "$REPO" &> /dev/null; then
        print_warning "Release $tag already exists. Deleting and recreating..."
        gh release delete "$tag" --repo "$REPO" --yes
    fi

    # Create release
    gh release create "$tag" \
        --repo "$REPO" \
        --title "agentvfs $VERSION" \
        --notes "## agentvfs $VERSION

Sandboxed filesystem for AI agents.

### Installation

\`\`\`bash
# Quick install
curl -sSfL https://raw.githubusercontent.com/neul-labs/agentvfs/main/scripts/install.sh | bash

# Or from crates.io
cargo install agentvfs
\`\`\`

### Downloads

| Platform | Architecture | Download |
|----------|--------------|----------|
| Linux | x86_64 | [avfs-${VERSION}-linux-x86_64.tar.gz](https://github.com/${REPO}/releases/download/${tag}/avfs-${VERSION}-linux-x86_64.tar.gz) |
| Linux | ARM64 | [avfs-${VERSION}-linux-aarch64.tar.gz](https://github.com/${REPO}/releases/download/${tag}/avfs-${VERSION}-linux-aarch64.tar.gz) |
| macOS | x86_64 | [avfs-${VERSION}-darwin-x86_64.tar.gz](https://github.com/${REPO}/releases/download/${tag}/avfs-${VERSION}-darwin-x86_64.tar.gz) |
| macOS | ARM64 | [avfs-${VERSION}-darwin-aarch64.tar.gz](https://github.com/${REPO}/releases/download/${tag}/avfs-${VERSION}-darwin-aarch64.tar.gz) |
| Windows | x86_64 | [avfs-${VERSION}-windows-x86_64.zip](https://github.com/${REPO}/releases/download/${tag}/avfs-${VERSION}-windows-x86_64.zip) |

### Changelog

See [CHANGELOG.md](https://github.com/${REPO}/blob/main/CHANGELOG.md) for details.
" \
        "${artifacts[@]}"

    print_success "Release $tag created: https://github.com/$REPO/releases/tag/$tag"
}

# Main function
main() {
    parse_args "$@"

    print_info "agentvfs Release Script"
    print_info "Version: $VERSION"
    print_info "Dry run: $DRY_RUN"

    # Change to repo root
    cd "$(dirname "$0")/.."

    verify_version
    check_requirements

    # Determine which targets to build
    local targets_to_build=()

    if [[ -n "$TARGETS" ]]; then
        IFS=',' read -ra requested_targets <<< "$TARGETS"
        for req in "${requested_targets[@]}"; do
            if target_info=$(get_target_info "$req"); then
                targets_to_build+=("$target_info")
            else
                print_error "Unknown target: $req"
                exit 1
            fi
        done
    else
        targets_to_build=("${ALL_TARGETS[@]}")
    fi

    print_info "Targets: ${#targets_to_build[@]}"

    # Build phase
    local artifacts=()

    if [[ "$SKIP_BUILD" == "false" ]]; then
        print_info "Starting builds..."

        for target in "${targets_to_build[@]}"; do
            IFS=':' read -r rust_target name ext <<< "$target"

            if build_target "$rust_target" "$name"; then
                artifact=$(package_target "$rust_target" "$name" "$ext")
                artifacts+=("$artifact")
            else
                print_warning "Skipping failed target: $name"
            fi
        done
    else
        print_info "Skipping build, using existing artifacts..."

        for target in "${targets_to_build[@]}"; do
            IFS=':' read -r rust_target name ext <<< "$target"
            local artifact_path="target/release-artifacts/${BINARY_NAME}-${VERSION}-${name}.${ext}"

            if [[ -f "$artifact_path" ]]; then
                artifacts+=("$artifact_path")
                print_success "Found: $artifact_path"
            else
                print_warning "Missing: $artifact_path"
            fi
        done
    fi

    if [[ ${#artifacts[@]} -eq 0 ]]; then
        print_error "No artifacts to release"
        exit 1
    fi

    print_info "Artifacts ready: ${#artifacts[@]}"
    for artifact in "${artifacts[@]}"; do
        echo "  - $artifact"
    done

    local checksum_file
    checksum_file=$(generate_checksums "${artifacts[@]}")
    artifacts+=("$checksum_file")

    # Release phase
    if [[ "$DRY_RUN" == "true" ]]; then
        print_warning "Dry run - skipping GitHub release"
        print_success "Dry run complete! Artifacts in target/release-artifacts/"
    else
        create_release "${artifacts[@]}"
    fi

    print_success "Done!"
}

main "$@"
