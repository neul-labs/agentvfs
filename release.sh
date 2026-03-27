#!/bin/bash
# AgentVFS (avfs) Release Script
#
# This script automates the release process:
# 1. Run tests
# 2. Build for all platforms
# 3. Create checksums
# 4. Create GitHub release
# 5. Optionally publish to crates.io
#
# Usage:
#   ./release.sh --version 0.2.0
#   ./release.sh --version 0.2.0 --dry-run
#   ./release.sh --version 0.2.0 --publish

set -e

# Configuration
REPO="neul-labs/agentvfs"
BINARY_NAME="avfs"

# Target platforms
TARGETS=(
    "x86_64-unknown-linux-gnu:linux-x86_64:tar.gz"
    "aarch64-unknown-linux-gnu:linux-aarch64:tar.gz"
    "x86_64-apple-darwin:darwin-x86_64:tar.gz"
    "aarch64-apple-darwin:darwin-aarch64:tar.gz"
    "x86_64-pc-windows-gnu:windows-x86_64:zip"
)

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

info() { echo -e "${BLUE}[INFO]${NC} $1"; }
success() { echo -e "${GREEN}[OK]${NC} $1"; }
warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
error() { echo -e "${RED}[ERROR]${NC} $1"; exit 1; }

# Show help
show_help() {
    cat << EOF
AgentVFS Release Script

USAGE:
    release.sh --version VERSION [OPTIONS]

OPTIONS:
    --version VERSION    Version to release (required)
    --dry-run            Build but don't upload to GitHub
    --skip-build         Skip building, use existing artifacts
    --skip-tests         Skip running tests
    --no-tag             Skip creating git tag (for re-releases)
    --publish            Also publish to crates.io
    --targets TARGETS    Comma-separated list of targets to build
    --help               Show this help message

TARGETS:
    x86_64-unknown-linux-gnu   Linux x86_64
    aarch64-unknown-linux-gnu  Linux ARM64
    x86_64-apple-darwin        macOS x86_64
    aarch64-apple-darwin       macOS ARM64 (Apple Silicon)
    x86_64-pc-windows-gnu      Windows x86_64

EXAMPLES:
    # Full release
    ./release.sh --version 0.2.0

    # Dry run (build only)
    ./release.sh --version 0.2.0 --dry-run

    # Release and publish to crates.io
    ./release.sh --version 0.2.0 --publish

    # Build only specific targets
    ./release.sh --version 0.2.0 --targets "x86_64-unknown-linux-gnu,x86_64-apple-darwin"
EOF
    exit 0
}

# Parse arguments
VERSION=""
DRY_RUN=false
SKIP_BUILD=false
SKIP_TESTS=false
SKIP_TAG=false
PUBLISH=false
CUSTOM_TARGETS=""

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
        --skip-tests)
            SKIP_TESTS=true
            shift
            ;;
        --no-tag)
            SKIP_TAG=true
            shift
            ;;
        --publish)
            PUBLISH=true
            shift
            ;;
        --targets)
            CUSTOM_TARGETS="$2"
            shift 2
            ;;
        --help|-h)
            show_help
            ;;
        *)
            error "Unknown option: $1"
            ;;
    esac
done

if [[ -z "$VERSION" ]]; then
    error "Version is required. Use --version VERSION"
fi

# Setup
DIST_DIR="dist"
ARTIFACTS_DIR="$DIST_DIR/artifacts"

# Check required tools
check_requirements() {
    info "Checking requirements..."

    if ! command -v cargo &> /dev/null; then
        error "cargo not found. Please install Rust."
    fi

    if ! command -v gh &> /dev/null; then
        error "gh (GitHub CLI) not found. Please install it: https://cli.github.com"
    fi

    if ! command -v cross &> /dev/null; then
        warn "cross not found. Installing..."
        cargo install cross
    fi

    if ! command -v tar &> /dev/null; then
        error "tar not found"
    fi

    success "All requirements met"
}

# Update version in Cargo.toml
update_version() {
    info "Updating version in Cargo.toml to ${VERSION}..."

    local current_version
    current_version=$(grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')

    if [[ "$current_version" == "$VERSION" ]]; then
        info "Version already set to ${VERSION}"
        return
    fi

    sed -i.bak "s/^version = \".*\"/version = \"${VERSION}\"/" Cargo.toml
    rm -f Cargo.toml.bak

    success "Version updated"
}

# Run tests
run_tests() {
    if [[ "$SKIP_TESTS" == true ]]; then
        warn "Skipping tests"
        return
    fi

    info "Running tests..."

    cargo test --all-features

    success "All tests passed"
}

# Build for a target
build_target() {
    local target="$1"
    local platform="$2"
    local ext="$3"

    info "Building for ${target}..."

    local build_dir="$ARTIFACTS_DIR/${platform}"
    mkdir -p "$build_dir"

    # Use cross for cross-compilation
    if [[ "$target" == *"linux"* && "$(uname -s)" != "Linux" ]] ||
       [[ "$target" == *"darwin"* && "$(uname -s)" != "Darwin" ]] ||
       [[ "$target" == *"windows"* && "$(uname -s)" != *"MINGW"* ]]; then
        cross build --release --target "$target"
    else
        cargo build --release --target "$target"
    fi

    # Copy binary
    local binary_name="$BINARY_NAME"
    if [[ "$target" == *"windows"* ]]; then
        binary_name="${BINARY_NAME}.exe"
    fi

    local binary_path="target/${target}/release/${binary_name}"
    if [[ ! -f "$binary_path" ]]; then
        error "Binary not found: $binary_path"
    fi

    cp "$binary_path" "$build_dir/"

    # Create archive
    local archive_name="${BINARY_NAME}-${VERSION}-${platform}"
    info "Creating archive: ${archive_name}.${ext}"

    pushd "$build_dir" > /dev/null

    if [[ "$ext" == "zip" ]]; then
        zip -q "../${archive_name}.zip" "$binary_name"
    else
        tar -czf "../${archive_name}.tar.gz" "$binary_name"
    fi

    popd > /dev/null

    success "Built ${platform}"
}

# Build all targets
build_all() {
    if [[ "$SKIP_BUILD" == true ]]; then
        warn "Skipping build"
        return
    fi

    info "Building for all platforms..."

    rm -rf "$DIST_DIR"
    mkdir -p "$ARTIFACTS_DIR"

    # Filter targets if custom list provided
    local targets_to_build=("${TARGETS[@]}")
    if [[ -n "$CUSTOM_TARGETS" ]]; then
        targets_to_build=()
        IFS=',' read -ra custom_list <<< "$CUSTOM_TARGETS"
        for target_spec in "${TARGETS[@]}"; do
            local target="${target_spec%%:*}"
            for custom in "${custom_list[@]}"; do
                if [[ "$target" == "$custom" ]]; then
                    targets_to_build+=("$target_spec")
                    break
                fi
            done
        done
    fi

    for target_spec in "${targets_to_build[@]}"; do
        IFS=':' read -r target platform ext <<< "$target_spec"
        build_target "$target" "$platform" "$ext" || warn "Failed to build ${target}"
    done

    success "All builds complete"
}

# Generate checksums
generate_checksums() {
    info "Generating checksums..."

    pushd "$ARTIFACTS_DIR" > /dev/null

    # Generate SHA256 checksums
    local checksum_file="checksums.txt"
    rm -f "$checksum_file"

    for archive in *.tar.gz *.zip 2>/dev/null; do
        if [[ -f "$archive" ]]; then
            if command -v sha256sum &> /dev/null; then
                sha256sum "$archive" >> "$checksum_file"
            elif command -v shasum &> /dev/null; then
                shasum -a 256 "$archive" >> "$checksum_file"
            fi
        fi
    done

    popd > /dev/null

    if [[ -f "$ARTIFACTS_DIR/$checksum_file" ]]; then
        success "Checksums generated"
        cat "$ARTIFACTS_DIR/$checksum_file"
    fi
}

# Create GitHub release
create_release() {
    if [[ "$DRY_RUN" == true ]]; then
        warn "Dry run - skipping GitHub release"
        info "Would create release v${VERSION} with files:"
        ls -la "$ARTIFACTS_DIR"/*.{tar.gz,zip,txt} 2>/dev/null || true
        return
    fi

    info "Creating GitHub release v${VERSION}..."

    # Check if release already exists
    if gh release view "v${VERSION}" &> /dev/null; then
        warn "Release v${VERSION} already exists"
        read -p "Delete and recreate? [y/N] " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            gh release delete "v${VERSION}" --yes
        else
            error "Release already exists"
        fi
    fi

    # Extract changelog or use default
    local changelog_notes
    changelog_notes=$(extract_changelog "$VERSION")

    local release_notes="## What's New in v${VERSION}
"
    if [[ -n "$changelog_notes" ]]; then
        release_notes="${release_notes}
${changelog_notes}"
    else
        release_notes="${release_notes}
### Features
- LMDB storage backend support (optional feature)
- Sled storage backend support (optional feature)
- Comprehensive test suite
- Improved CI/CD pipeline
"
    fi

    release_notes="${release_notes}
### Installation

\`\`\`bash
curl -fsSL https://raw.githubusercontent.com/${REPO}/main/install.sh | bash
\`\`\`

Or with cargo:
\`\`\`bash
cargo install agentvfs
\`\`\`

### Available Features

- \`sled-backend\`: Sled storage with Tantivy search
- \`lmdb-backend\`: LMDB storage with Tantivy search
- \`fuse\`: FUSE filesystem mounting

Install with features:
\`\`\`bash
cargo install agentvfs --features \"sled-backend,lmdb-backend\"
\`\`\`

### Checksums
See checksums.txt for SHA256 checksums of all artifacts.
"

    # Create release
    gh release create "v${VERSION}" \
        --title "v${VERSION}" \
        --notes "$release_notes" \
        "$ARTIFACTS_DIR"/*.tar.gz \
        "$ARTIFACTS_DIR"/*.zip \
        "$ARTIFACTS_DIR"/checksums.txt \
        2>/dev/null || {
            # If some files don't exist, try without patterns
            gh release create "v${VERSION}" \
                --title "v${VERSION}" \
                --notes "$release_notes" \
                $(ls "$ARTIFACTS_DIR"/*.tar.gz "$ARTIFACTS_DIR"/*.zip "$ARTIFACTS_DIR"/checksums.txt 2>/dev/null)
        }

    success "GitHub release created: https://github.com/${REPO}/releases/tag/v${VERSION}"
}

# Publish to crates.io
publish_crate() {
    if [[ "$PUBLISH" != true ]]; then
        info "Skipping crates.io publish (use --publish to enable)"
        return
    fi

    if [[ "$DRY_RUN" == true ]]; then
        warn "Dry run - would publish to crates.io"
        cargo publish --dry-run
        return
    fi

    info "Publishing to crates.io..."

    cargo publish

    success "Published to crates.io"
}

# Extract release notes from CHANGELOG.md
extract_changelog() {
    local version="$1"
    local in_section=false
    local notes=""

    if [[ ! -f "CHANGELOG.md" ]]; then
        echo ""
        return
    fi

    while IFS= read -r line; do
        # Check for version header (## [version] or ## version)
        if [[ "$line" =~ ^##[[:space:]]+\[?${version}\]? ]] || [[ "$line" =~ ^##[[:space:]]+v?${version} ]]; then
            in_section=true
            continue
        fi

        # Check for next version header (end of section)
        if [[ "$in_section" == true ]] && [[ "$line" =~ ^##[[:space:]] ]]; then
            break
        fi

        # Collect notes
        if [[ "$in_section" == true ]]; then
            notes="${notes}${line}
"
        fi
    done < "CHANGELOG.md"

    echo "$notes"
}

# Create git tag
create_tag() {
    if [[ "$DRY_RUN" == true ]]; then
        warn "Dry run - skipping git tag"
        return
    fi

    if [[ "$SKIP_TAG" == true ]]; then
        warn "Skipping git tag (--no-tag specified)"
        return
    fi

    if git rev-parse "v${VERSION}" &> /dev/null; then
        warn "Tag v${VERSION} already exists"
        return
    fi

    info "Creating git tag v${VERSION}..."

    git tag -a "v${VERSION}" -m "Release v${VERSION}"
    git push origin "v${VERSION}"

    success "Tag created and pushed"
}

# Main
main() {
    echo ""
    echo "  AgentVFS Release Script"
    echo "  ======================="
    echo ""
    echo "  Version: ${VERSION}"
    echo "  Dry run: ${DRY_RUN}"
    echo ""

    check_requirements
    update_version
    run_tests
    build_all
    generate_checksums
    create_tag
    create_release
    publish_crate

    echo ""
    success "Release v${VERSION} complete!"
    echo ""
}

main
