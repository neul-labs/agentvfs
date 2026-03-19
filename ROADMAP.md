# vfs Roadmap

## Project Status: Documentation Complete, Implementation Pending

This document outlines the implementation roadmap for vfs, a virtual filesystem CLI backed by embedded databases.

---

## Phase 1: Core Foundation

**Goal:** Minimal viable filesystem with SQLite backend

### 1.1 Project Setup
- [ ] Initialize Rust project with Cargo
- [ ] Set up project structure (src/lib.rs, src/main.rs, src/commands/, src/storage/)
- [ ] Add dependencies: clap, rusqlite, serde, sha2, thiserror
- [ ] Set up error types and Result aliases

### 1.2 Storage Backend Trait
- [ ] Define `StorageBackend` trait (get, put, delete, scan, transaction)
- [ ] Define `SearchBackend` trait (index, search, remove)
- [ ] Implement SQLite backend
- [ ] Implement SQLite FTS5 search backend

### 1.3 Core Data Structures
- [ ] FileEntry, ContentBlob, VersionEntry structs
- [ ] Serialization with bincode/serde
- [ ] Path normalization utilities
- [ ] SHA-256 content hashing

### 1.4 Vault Management
- [ ] `vfs vault create` - create new vault
- [ ] `vfs vault list` - list vaults
- [ ] `vfs vault use` - switch active vault
- [ ] `vfs vault delete` - delete vault
- [ ] `vfs vault info` - show vault info
- [ ] Global config file (~/.vfs/config.toml)

### 1.5 Basic File Operations
- [ ] `vfs ls` - list directory
- [ ] `vfs mkdir` - create directory
- [ ] `vfs rmdir` - remove empty directory
- [ ] `vfs touch` - create empty file
- [ ] `vfs write` - write content to file
- [ ] `vfs cat` - read file content
- [ ] `vfs cp` - copy file/directory
- [ ] `vfs mv` - move/rename file
- [ ] `vfs rm` - remove file/directory
- [ ] `vfs pwd` - print working directory
- [ ] `vfs cd` - change directory
- [ ] `vfs tree` - display tree

---

## Phase 2: Versioning & Search

**Goal:** Automatic versioning and content search

### 2.1 Automatic Versioning
- [ ] Create version on every write
- [ ] `vfs log` - show version history
- [ ] `vfs cat -v <N>` - read specific version
- [ ] `vfs checkout` - restore version
- [ ] `vfs revert` - revert to previous
- [ ] `vfs diff` - compare files/versions

### 2.2 Search
- [ ] FTS5 index management
- [ ] `vfs search` - full-text search
- [ ] `vfs grep` - regex content search
- [ ] `vfs find` - find by name/attributes

---

## Phase 3: Metadata & Tags

**Goal:** Rich file organization

### 3.1 Tags
- [ ] `vfs tag` - add tags to files
- [ ] `vfs untag` - remove tags
- [ ] `vfs tag --list` - list all tags
- [ ] `vfs tag --create/--delete/--rename`
- [ ] `vfs find -tag` - find by tag

### 3.2 Custom Metadata
- [ ] `vfs meta` - get/set metadata
- [ ] `vfs meta --export/--import`
- [ ] `vfs find -meta` - find by metadata

---

## Phase 4: Import/Export & External Commands

**Goal:** Bridge to real filesystem

### 4.1 Import/Export
- [ ] `vfs import` - import from real filesystem
- [ ] `vfs export` - export to real filesystem
- [ ] Recursive import/export

### 4.2 External Commands
- [ ] `vfs exec` - run command on virtual file
- [ ] Temp file extraction and re-import
- [ ] Glob pattern support
- [ ] Pipe support (`vfs cat | cmd | vfs write`)

---

## Phase 5: Maintenance

**Goal:** Storage management and optimization

### 5.1 Pruning
- [ ] `vfs prune --keep <N>` - keep last N versions
- [ ] `vfs prune --older-than <DAYS>` - time-based
- [ ] `vfs prune --max-size <MB>` - size-based
- [ ] `vfs vault config` - configure prune defaults

### 5.2 Garbage Collection & Compaction
- [ ] `vfs gc` - remove orphaned blobs
- [ ] `vfs compact` - reclaim space (VACUUM)
- [ ] `vfs maintain` - full maintenance routine
- [ ] `vfs vault stats` - storage statistics

---

## Phase 6: Agent Integration

**Goal:** AI agent sandbox support

### 6.1 JSON Output
- [ ] `--json` flag on all commands
- [ ] Consistent JSON error format
- [ ] Structured output for parsing

### 6.2 Snapshots
- [ ] `vfs snapshot save` - save vault state
- [ ] `vfs snapshot list` - list snapshots
- [ ] `vfs snapshot restore` - restore state
- [ ] `vfs snapshot delete` - delete snapshot

### 6.3 Quotas
- [ ] `max_size_mb` limit
- [ ] `max_files` limit
- [ ] `max_file_size_mb` limit
- [ ] Quota enforcement on write operations

### 6.4 Audit Log
- [ ] Log all operations to vault
- [ ] `vfs audit` - view operation history
- [ ] `vfs audit clear` - clear log
- [ ] Auto-rotation at max entries

---

## Phase 7: Interactive Shell

**Goal:** REPL experience

- [ ] `vfs shell` - launch interactive mode
- [ ] Command parsing without `vfs` prefix
- [ ] Custom prompt with vault/path
- [ ] Tab completion (rustyline)
- [ ] Command history
- [ ] `vfs aliases` - generate shell aliases

---

## Phase 8: Additional Backends

**Goal:** Pluggable storage options

### 8.1 Sled Backend
- [ ] Implement `StorageBackend` for Sled
- [ ] Tantivy search integration

### 8.2 LMDB Backend
- [ ] Implement `StorageBackend` for LMDB
- [ ] Tantivy search integration

### 8.3 RocksDB Backend
- [ ] Implement `StorageBackend` for RocksDB
- [ ] Tantivy search integration

### 8.4 Backend Migration
- [ ] `vfs vault migrate --to <backend>`
- [ ] Data verification after migration

---

## Phase 9: Polish & Distribution

**Goal:** Production-ready release

### 9.1 Testing
- [ ] Unit tests for all commands
- [ ] Integration tests
- [ ] Fuzzing for parser/storage
- [ ] Benchmark suite

### 9.2 Documentation
- [x] README.md
- [x] docs/*.md (all documentation)
- [ ] Man pages
- [ ] `--help` text for all commands

### 9.3 Distribution
- [ ] Publish to crates.io
- [ ] GitHub releases with binaries
- [ ] Homebrew formula
- [ ] AUR package

---

## Future Considerations (Not Planned)

These are explicitly out of scope for initial release:

| Feature | Reason |
|---------|--------|
| REST API | CLI + JSON is sufficient for agents |
| Permissions/ACL | Overkill for single-agent use |
| Multi-agent workspaces | Premature; one vault per agent works |
| FUSE mount | Heavy dependency |
| Encryption at rest | Can add later |
| Network sync | Complex; use file copy for now |

---

## Implementation Notes

### Recommended Crates

| Purpose | Crate |
|---------|-------|
| CLI parsing | clap |
| SQLite | rusqlite |
| Sled | sled |
| LMDB | heed |
| RocksDB | rocksdb |
| Serialization | serde, bincode |
| Hashing | sha2 |
| Regex | regex |
| Full-text search | tantivy |
| REPL | rustyline |
| Error handling | thiserror, anyhow |
| JSON | serde_json |
| Time | chrono |
| Glob patterns | glob |

### Project Structure

```
vfs/
├── Cargo.toml
├── src/
│   ├── main.rs              # CLI entry point
│   ├── lib.rs               # Library root
│   ├── error.rs             # Error types
│   ├── commands/
│   │   ├── mod.rs
│   │   ├── ls.rs
│   │   ├── cat.rs
│   │   ├── write.rs
│   │   └── ...
│   ├── storage/
│   │   ├── mod.rs
│   │   ├── backend.rs       # Trait definitions
│   │   ├── sqlite.rs
│   │   ├── sled.rs
│   │   └── ...
│   ├── vault/
│   │   ├── mod.rs
│   │   ├── config.rs
│   │   └── snapshot.rs
│   └── shell/
│       ├── mod.rs
│       └── repl.rs
└── tests/
    ├── integration/
    └── fixtures/
```

---

## Success Metrics

- [ ] All commands work as documented
- [ ] JSON output parseable by Python/JS
- [ ] Snapshots save/restore correctly
- [ ] Quotas prevent runaway usage
- [ ] Backend migration preserves all data
- [ ] Interactive shell is responsive
- [ ] No data loss under any circumstance
