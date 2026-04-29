#!/bin/bash
# AgentVFS (avfs) Release Script
#
# All Rust building, crate publishing, and GitHub release creation happens in
# .github/workflows/release.yml when a v* tag is pushed. This script handles
# only the local steps:
#
#   1. Bump the version in Cargo.toml
#   2. (Optional) Run the test suite as a pre-tag sanity gate
#   3. Create + push the v${VERSION} tag (which triggers CI)
#   4. Wait for the CI-built GitHub release to have all platform artifacts
#   5. Publish the npm and PyPI wrapper packages (which download those artifacts
#      at install time, so they MUST come after CI finishes)
#
# Usage:
#   ./release.sh --version 0.2.0 --publish-npm --publish-pypi
#   ./release.sh --version 0.2.0 --dry-run
#   ./release.sh --version 0.2.0 --skip-wait --publish-npm   # release already exists

set -e

REPO="neul-labs/agentvfs"
BINARY_NAME="avfs"

# Files the CI workflow must publish to the GitHub release before we publish
# the npm/PyPI wrappers. Keep this list in sync with the build-release matrix
# in .github/workflows/release.yml.
EXPECTED_ARCHIVES=(
    "linux-x86_64.tar.gz"
    "linux-aarch64.tar.gz"
    "darwin-x86_64.tar.gz"
    "darwin-aarch64.tar.gz"
    "windows-x86_64.zip"
)

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

info()    { echo -e "${BLUE}[INFO]${NC} $1"; }
success() { echo -e "${GREEN}[OK]${NC} $1"; }
warn()    { echo -e "${YELLOW}[WARN]${NC} $1"; }
error()   { echo -e "${RED}[ERROR]${NC} $1"; exit 1; }

show_help() {
    cat << EOF
AgentVFS Release Script

USAGE:
    release.sh --version VERSION [OPTIONS]

OPTIONS:
    --version VERSION    Version to release (required)
    --dry-run            Don't tag, push, or publish — print what would happen
    --skip-tests         Skip running 'cargo test' before tagging
    --no-tag             Don't create/push a tag (use when re-running publishes)
    --skip-wait          Don't wait for the CI release to populate (assumes done)
    --publish-npm        Publish the npm wrapper after the CI release is ready
    --publish-pypi       Publish the PyPI wrapper after the CI release is ready
    --help               Show this help message

CI HANDLES:
    - Building all platform binaries (linux/darwin/windows, x86_64/aarch64)
    - Creating the GitHub release and uploading archives + checksums
    - Publishing to crates.io

LOCAL HANDLES:
    - Version bump, tag/push, npm wrapper publish, PyPI wrapper publish

EXAMPLES:
    # Full local-driven release: tag, wait for CI, publish wrappers
    ./release.sh --version 0.2.0 --publish-npm --publish-pypi

    # Just cut the tag and let CI run; publish wrappers later
    ./release.sh --version 0.2.0

    # CI release already exists — only push the npm wrapper
    ./release.sh --version 0.2.0 --no-tag --skip-wait --publish-npm
EOF
    exit 0
}

VERSION=""
DRY_RUN=false
SKIP_TESTS=false
SKIP_TAG=false
SKIP_WAIT=false
PUBLISH_NPM=false
PUBLISH_PYPI=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --version)      VERSION="$2"; shift 2 ;;
        --dry-run)      DRY_RUN=true; shift ;;
        --skip-tests)   SKIP_TESTS=true; shift ;;
        --no-tag)       SKIP_TAG=true; shift ;;
        --skip-wait)    SKIP_WAIT=true; shift ;;
        --publish-npm)  PUBLISH_NPM=true; shift ;;
        --publish-pypi) PUBLISH_PYPI=true; shift ;;
        --help|-h)      show_help ;;
        *)              error "Unknown option: $1" ;;
    esac
done

[[ -z "$VERSION" ]] && error "Version is required. Use --version VERSION"

check_requirements() {
    info "Checking requirements..."
    command -v git  &>/dev/null || error "git not found"
    command -v gh   &>/dev/null || error "gh (GitHub CLI) not found: https://cli.github.com"
    command -v cargo &>/dev/null || error "cargo not found (needed for tests + version bump)"
    if [[ "$PUBLISH_NPM" == true ]]; then
        command -v npm &>/dev/null || error "npm not found (needed for --publish-npm)"
    fi
    if [[ "$PUBLISH_PYPI" == true ]]; then
        command -v python3 &>/dev/null || command -v python &>/dev/null \
            || error "python not found (needed for --publish-pypi)"
        command -v twine &>/dev/null || error "twine not found (pip install twine)"
    fi
    success "All requirements met"
}

update_version() {
    info "Updating Cargo.toml version to ${VERSION}..."
    local current
    current=$(grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')
    if [[ "$current" == "$VERSION" ]]; then
        info "Version already set to ${VERSION}"
        return
    fi
    sed -i.bak "s/^version = \".*\"/version = \"${VERSION}\"/" Cargo.toml
    rm -f Cargo.toml.bak
    success "Cargo.toml updated"
}

run_tests() {
    if [[ "$SKIP_TESTS" == true ]]; then
        warn "Skipping tests"
        return
    fi
    info "Running tests..."
    cargo test --all-features
    success "All tests passed"
}

create_tag() {
    if [[ "$SKIP_TAG" == true ]]; then
        warn "Skipping git tag (--no-tag)"
        return
    fi
    if [[ "$DRY_RUN" == true ]]; then
        warn "Dry run — would tag and push v${VERSION}"
        return
    fi
    if git rev-parse "v${VERSION}" &>/dev/null; then
        warn "Tag v${VERSION} already exists locally"
    else
        info "Creating git tag v${VERSION}..."
        git tag -a "v${VERSION}" -m "Release v${VERSION}"
    fi
    info "Pushing tag v${VERSION} to origin (this fires release.yml)..."
    git push origin "v${VERSION}"
    success "Tag pushed — watch CI: gh run watch  (or gh run list --workflow=release.yml)"
}

# Block until the GitHub release v${VERSION} exists with every expected
# platform archive uploaded by the CI build matrix. Required before we publish
# the npm/PyPI wrappers, whose install scripts download these archives by URL.
wait_for_github_release() {
    if [[ "$SKIP_WAIT" == true ]]; then
        warn "Skipping CI wait (--skip-wait)"
        return
    fi
    if [[ "$DRY_RUN" == true ]]; then
        warn "Dry run — would wait for v${VERSION} artifacts on GitHub"
        return
    fi
    if [[ "$PUBLISH_NPM" != true && "$PUBLISH_PYPI" != true ]]; then
        info "No wrapper publish requested — skipping CI wait"
        return
    fi

    local expected=()
    for suffix in "${EXPECTED_ARCHIVES[@]}"; do
        expected+=("${BINARY_NAME}-${VERSION}-${suffix}")
    done

    info "Waiting for v${VERSION} on GitHub to have ${#expected[@]} platform artifacts..."

    local timeout_secs=1800   # 30 minutes — release.yml typically finishes in ~10–15
    local interval_secs=20
    local elapsed=0

    while (( elapsed < timeout_secs )); do
        local assets
        if assets=$(gh release view "v${VERSION}" --json assets --jq '.assets[].name' 2>/dev/null); then
            local missing=()
            for f in "${expected[@]}"; do
                grep -qx "$f" <<< "$assets" || missing+=("$f")
            done
            if (( ${#missing[@]} == 0 )); then
                success "All ${#expected[@]} platform artifacts present on v${VERSION}"
                return
            fi
            info "Still waiting on ${#missing[@]} artifact(s): ${missing[*]}"
        else
            info "Release v${VERSION} not visible yet..."
        fi
        sleep "$interval_secs"
        elapsed=$((elapsed + interval_secs))
    done

    error "Timed out after ${timeout_secs}s. Check: gh run list --workflow=release.yml"
}

publish_npm() {
    if [[ "$PUBLISH_NPM" != true ]]; then
        info "Skipping npm publish (use --publish-npm to enable)"
        return
    fi
    if [[ "$DRY_RUN" == true ]]; then
        warn "Dry run — would publish npm wrapper @ ${VERSION}"
        return
    fi
    info "Publishing npm wrapper..."
    pushd packaging/npm > /dev/null
    npm version "${VERSION}" --no-git-tag-version
    npm publish --access public
    popd > /dev/null
    success "Published npm wrapper"
}

publish_pypi() {
    if [[ "$PUBLISH_PYPI" != true ]]; then
        info "Skipping PyPI publish (use --publish-pypi to enable)"
        return
    fi
    if [[ "$DRY_RUN" == true ]]; then
        warn "Dry run — would publish PyPI wrapper @ ${VERSION}"
        return
    fi
    info "Publishing PyPI wrapper..."
    pushd packaging/pypi > /dev/null
    sed -i.bak "s/^version = \"[^\"]*\"/version = \"${VERSION}\"/" pyproject.toml
    rm -f pyproject.toml.bak
    rm -rf dist
    python -m build
    twine upload dist/*
    popd > /dev/null
    success "Published PyPI wrapper"
}

main() {
    echo ""
    echo "  AgentVFS Release Script"
    echo "  ======================="
    echo "  Version:       ${VERSION}"
    echo "  Dry run:       ${DRY_RUN}"
    echo "  Publish npm:   ${PUBLISH_NPM}"
    echo "  Publish pypi:  ${PUBLISH_PYPI}"
    echo ""

    check_requirements
    update_version
    run_tests
    create_tag
    wait_for_github_release
    publish_npm
    publish_pypi

    echo ""
    success "Release v${VERSION} complete!"
    echo ""
}

main
