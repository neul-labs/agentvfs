# Changelog

All notable changes to AgentVFS (avfs) will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- LMDB storage backend with Tantivy full-text search (optional feature: `lmdb-backend`)
- Sled storage backend with Tantivy full-text search (optional feature: `sled-backend`)
- Comprehensive test suite with unit and integration tests
- GitHub Actions CI/CD pipelines for automated testing and releases
- Root-level `install.sh` script for easy installation with `--features` and `--quiet` options
- Root-level `release.sh` script for release automation with `--no-tag` option
- Man page documentation (`man/avfs.1`)
- Homebrew formula (`packaging/homebrew/avfs.rb`)
- AUR PKGBUILD (`packaging/aur/PKGBUILD`)
- `CONTRIBUTING.md` with contribution guidelines

### Changed
- Updated documentation for new backend support
- Improved error handling with Sled, LMDB, and Tantivy error types
- Enhanced install.sh with feature selection and quiet mode
- Enhanced release.sh with CHANGELOG extraction for release notes

## [0.1.0] - 2024-01-15

### Added
- Initial release of AgentVFS
- SQLite storage backend with WAL mode
- Full-text search using FTS5
- Core file operations: ls, mkdir, rmdir, touch, write, cat, cp, mv, rm, pwd, tree
- Automatic versioning on every write
- Version history: log, checkout, revert, diff
- Content search: search, grep, find
- Tags system: tag, untag, tag management
- Custom metadata: meta get/set
- Import/Export: import, export with recursive support
- External commands: exec with temp file extraction
- Maintenance: prune, gc, compact, maintain, stats
- Agent integration: --json flag, snapshots, quotas, audit log
- Interactive shell with tab completion and history
- FUSE mount support (optional feature)

### Security
- Symlink security checks on import
- Quota enforcement to prevent runaway usage

[Unreleased]: https://github.com/neul-labs/agentvfs/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/neul-labs/agentvfs/releases/tag/v0.1.0
