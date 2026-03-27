# vfs Roadmap

## Project Status: Core Implementation Complete (Phases 1-7)

This document outlines the implementation roadmap for vfs, a virtual filesystem CLI backed by embedded databases.

---

## Phase 1: Core Foundation

**Goal:** Minimal viable filesystem with SQLite backend

### 1.1 Project Setup
- [x] Initialize Rust project with Cargo
- [x] Set up project structure (src/lib.rs, src/main.rs, src/commands/, src/storage/)
- [x] Add dependencies: clap, rusqlite, serde, sha2, thiserror
- [x] Set up error types and Result aliases

### 1.2 Storage Backend Trait
- [x] Define `StorageBackend` trait (get, put, delete, scan, transaction)
- [x] Define `SearchBackend` trait (index, search, remove)
- [x] Implement SQLite backend
- [x] Implement SQLite FTS5 search backend

### 1.3 Core Data Structures
- [x] FileEntry, ContentBlob, VersionEntry structs
- [x] Serialization with bincode/serde
- [x] Path normalization utilities
- [x] SHA-256 content hashing

### 1.4 Vault Management
- [x] `vfs vault create` - create new vault
- [x] `vfs vault list` - list vaults
- [x] `vfs vault use` - switch active vault
- [x] `vfs vault delete` - delete vault
- [x] `vfs vault info` - show vault info
- [x] Global config file (~/.vfs/config.toml)

### 1.5 Basic File Operations
- [x] `vfs ls` - list directory
- [x] `vfs mkdir` - create directory
- [x] `vfs rmdir` - remove empty directory
- [x] `vfs touch` - create empty file
- [x] `vfs write` - write content to file
- [x] `vfs cat` - read file content
- [x] `vfs cp` - copy file/directory
- [x] `vfs mv` - move/rename file
- [x] `vfs rm` - remove file/directory
- [x] `vfs pwd` - print working directory
- [x] `vfs cd` - change directory
- [x] `vfs tree` - display tree

---

## Phase 2: Versioning & Search

**Goal:** Automatic versioning and content search

### 2.1 Automatic Versioning
- [x] Create version on every write
- [x] `vfs log` - show version history
- [x] `vfs cat -v <N>` - read specific version
- [x] `vfs checkout` - restore version
- [x] `vfs revert` - revert to previous
- [x] `vfs diff` - compare files/versions

### 2.2 Search
- [x] FTS5 index management
- [x] `vfs search` - full-text search
- [x] `vfs grep` - regex content search
- [x] `vfs find` - find by name/attributes

---

## Phase 3: Metadata & Tags

**Goal:** Rich file organization

### 3.1 Tags
- [x] `vfs tag` - add tags to files
- [x] `vfs untag` - remove tags
- [x] `vfs tag --list` - list all tags
- [x] `vfs tag --create/--delete/--rename`
- [x] `vfs find -tag` - find by tag

### 3.2 Custom Metadata
- [x] `vfs meta` - get/set metadata
- [x] `vfs meta --export/--import`
- [x] `vfs find -meta` - find by metadata

---

## Phase 4: Import/Export & External Commands

**Goal:** Bridge to real filesystem

### 4.1 Import/Export
- [x] `vfs import` - import from real filesystem
- [x] `vfs export` - export to real filesystem
- [x] Recursive import/export

### 4.2 External Commands
- [x] `vfs exec` - run command on virtual file
- [x] Temp file extraction and re-import
- [x] Glob pattern support
- [x] Pipe support (`vfs cat | cmd | vfs write`)

---

## Phase 5: Maintenance

**Goal:** Storage management and optimization

### 5.1 Pruning
- [x] `vfs prune --keep <N>` - keep last N versions
- [x] `vfs prune --older-than <DAYS>` - time-based
- [x] `vfs prune --max-size <MB>` - size-based
- [x] `vfs vault config` - configure prune defaults

### 5.2 Garbage Collection & Compaction
- [x] `vfs gc` - remove orphaned blobs
- [x] `vfs compact` - reclaim space (VACUUM)
- [x] `vfs maintain` - full maintenance routine
- [x] `vfs vault stats` - storage statistics

---

## Phase 6: Agent Integration

**Goal:** AI agent sandbox support

### 6.1 JSON Output
- [x] `--json` flag on all commands
- [x] Consistent JSON error format
- [x] Structured output for parsing

### 6.2 Snapshots
- [x] `vfs snapshot save` - save vault state
- [x] `vfs snapshot list` - list snapshots
- [x] `vfs snapshot restore` - restore state
- [x] `vfs snapshot delete` - delete snapshot

### 6.3 Quotas
- [x] `max_size_mb` limit
- [x] `max_files` limit
- [x] `max_file_size_mb` limit
- [x] Quota enforcement on write operations

### 6.4 Audit Log
- [x] Log all operations to vault
- [x] `vfs audit` - view operation history
- [x] `vfs audit clear` - clear log
- [x] Auto-rotation at max entries

---

## Phase 7: Interactive Shell

**Goal:** REPL experience

- [x] `vfs shell` - launch interactive mode
- [x] Command parsing without `vfs` prefix
- [x] Custom prompt with vault/path
- [x] Tab completion (rustyline)
- [x] Command history
- [x] `vfs aliases` - generate shell aliases

---

## Phase 8: Additional Backends

**Goal:** Pluggable storage options

### 8.1 Sled Backend
- [x] Implement `StorageBackend` for Sled
- [x] Tantivy search integration

### 8.2 LMDB Backend
- [x] Implement `StorageBackend` for LMDB
- [x] Tantivy search integration

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
- [x] Unit tests for all commands
- [x] Integration tests
- [ ] Fuzzing for parser/storage
- [ ] Benchmark suite

### 9.2 Documentation
- [x] README.md
- [x] docs/*.md (all documentation)
- [x] Man pages
- [x] `--help` text for all commands

### 9.3 Distribution
- [ ] Publish to crates.io
- [x] GitHub releases with binaries
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

- [x] All commands work as documented
- [x] JSON output parseable by Python/JS
- [x] Snapshots save/restore correctly
- [x] Quotas prevent runaway usage
- [ ] Backend migration preserves all data
- [x] Interactive shell is responsive
- [x] No data loss under any circumstance
