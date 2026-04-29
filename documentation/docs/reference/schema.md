# Database Schema

This document describes the logical data model used by VFS. The model is backend-agnostic, with specific implementations for each storage backend.

## Logical Data Model

VFS organizes data into **collections** (similar to tables or trees). Each collection stores key-value pairs with specific semantics.

### Collections Overview

| Collection | Purpose | Key Format | Value Type |
|------------|---------|------------|------------|
| `files` | File/directory metadata | file_id (u64) | FileEntry |
| `paths` | Path to ID mapping | path (string) | file_id (u64) |
| `contents` | Content blobs (CAS) | hash (32 bytes) | ContentBlob |
| `versions` | Version history | file_id + version_num | VersionEntry |
| `tags` | Tag definitions | tag_id (u64) | TagInfo |
| `tag_names` | Tag name lookup | tag_name (string) | tag_id (u64) |
| `file_tags` | File-tag associations | file_id + tag_id | timestamp |
| `file_meta` | Custom metadata | file_id + key | value (string) |
| `settings` | Vault configuration | key (string) | value (string) |

---

## Data Structures

### FileEntry

Represents a file or directory in the virtual filesystem.

```rust
struct FileEntry {
    id: u64,
    parent_id: Option<u64>,     // None for root
    name: String,
    file_type: FileType,        // File or Directory
    content_hash: Option<Hash>, // None for directories
    size: u64,
    created_at: Timestamp,
    modified_at: Timestamp,
}

enum FileType {
    File,
    Directory,
}
```

**Serialized format** (bincode/MessagePack):

```
[id: 8 bytes][parent_id: 9 bytes][name: var][type: 1 byte]
[hash: 33 bytes][size: 8 bytes][created: 8 bytes][modified: 8 bytes]
```

### ContentBlob

Stores actual file content using content-addressable storage.

```rust
struct ContentBlob {
    hash: [u8; 32],      // SHA-256
    data: Vec<u8>,       // Raw content
    size: u64,
    ref_count: u32,      // Reference counting for GC
    created_at: Timestamp,
}
```

### VersionEntry

Records a point-in-time snapshot of a file.

```rust
struct VersionEntry {
    file_id: u64,
    version_num: u32,
    content_hash: Hash,
    size: u64,
    created_at: Timestamp,
}
```

**Key format**: `file_id (8 bytes) + version_num (4 bytes)`

### TagInfo

Defines a tag that can be applied to files.

```rust
struct TagInfo {
    id: u64,
    name: String,
    color: Option<String>,  // Hex color code
    created_at: Timestamp,
}
```

---

## Key Encoding

Keys are encoded consistently across backends:

### Numeric Keys

- u64 values are encoded as big-endian bytes for proper ordering
- Example: `1000` → `[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03, 0xE8]`

### String Keys

- UTF-8 encoded
- Paths use forward slashes, no trailing slash (except root)

### Composite Keys

- Components are length-prefixed or use fixed sizes
- Example file_tag key: `file_id (8 bytes) + tag_id (8 bytes)`

```rust
fn encode_file_tag_key(file_id: u64, tag_id: u64) -> [u8; 16] {
    let mut key = [0u8; 16];
    key[0..8].copy_from_slice(&file_id.to_be_bytes());
    key[8..16].copy_from_slice(&tag_id.to_be_bytes());
    key
}
```

---

## Value Serialization

Values are serialized using a compact binary format:

### Primary: bincode

- Fast, compact, Rust-native
- Used for internal structures

### Alternative: MessagePack

- More portable
- Better for potential cross-language access

### Configuration

```toml
[storage]
serialization = "bincode"  # or "msgpack", "cbor"
```

---

## Backend-Specific Implementations

### SQLite Schema

When using SQLite, collections map to tables:

```sql
-- File metadata
CREATE TABLE files (
    id              INTEGER PRIMARY KEY,
    parent_id       INTEGER REFERENCES files(id) ON DELETE CASCADE,
    name            TEXT NOT NULL,
    file_type       INTEGER NOT NULL,  -- 0=file, 1=directory
    content_hash    BLOB,
    size            INTEGER NOT NULL DEFAULT 0,
    created_at      INTEGER NOT NULL,  -- Unix timestamp
    modified_at     INTEGER NOT NULL,
    UNIQUE(parent_id, name)
);

CREATE INDEX idx_files_parent ON files(parent_id);
CREATE INDEX idx_files_hash ON files(content_hash);

-- Path lookup (denormalized for performance)
CREATE TABLE paths (
    path    TEXT PRIMARY KEY,
    file_id INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE
);

-- Content-addressable storage
CREATE TABLE contents (
    hash        BLOB PRIMARY KEY,  -- 32 bytes SHA-256
    data        BLOB NOT NULL,
    size        INTEGER NOT NULL,
    ref_count   INTEGER NOT NULL DEFAULT 1,
    created_at  INTEGER NOT NULL
);

-- Version history
CREATE TABLE versions (
    file_id     INTEGER NOT NULL,
    version_num INTEGER NOT NULL,
    content_hash BLOB NOT NULL,
    size        INTEGER NOT NULL,
    created_at  INTEGER NOT NULL,
    PRIMARY KEY (file_id, version_num)
);

CREATE INDEX idx_versions_created ON versions(created_at);

-- Tags
CREATE TABLE tags (
    id          INTEGER PRIMARY KEY,
    name        TEXT NOT NULL UNIQUE,
    color       TEXT,
    created_at  INTEGER NOT NULL
);

CREATE TABLE file_tags (
    file_id     INTEGER NOT NULL,
    tag_id      INTEGER NOT NULL,
    created_at  INTEGER NOT NULL,
    PRIMARY KEY (file_id, tag_id)
);

CREATE INDEX idx_file_tags_tag ON file_tags(tag_id);

-- Custom metadata
CREATE TABLE file_meta (
    file_id     INTEGER NOT NULL,
    key         TEXT NOT NULL,
    value       TEXT NOT NULL,
    PRIMARY KEY (file_id, key)
);

-- Full-text search (FTS5)
CREATE VIRTUAL TABLE fts_content USING fts5(
    path,
    content,
    content_rowid='file_id'
);

-- Vault settings
CREATE TABLE settings (
    key     TEXT PRIMARY KEY,
    value   TEXT NOT NULL
);
```

### Sled/LMDB/RocksDB Schema

For key-value backends, collections map to separate trees/databases:

```
Tree: files
  Key: u64 (big-endian)
  Value: bincode(FileEntry)

Tree: paths
  Key: UTF-8 string
  Value: u64 (big-endian)

Tree: contents
  Key: 32 bytes (SHA-256)
  Value: raw bytes (content data)

Tree: contents_meta
  Key: 32 bytes (SHA-256)
  Value: bincode(ContentMeta { size, ref_count, created_at })

Tree: versions
  Key: file_id (8 bytes) + version_num (4 bytes)
  Value: bincode(VersionEntry)

Tree: tags
  Key: u64 (big-endian)
  Value: bincode(TagInfo)

Tree: tag_names
  Key: UTF-8 string
  Value: u64 (big-endian)

Tree: file_tags
  Key: file_id (8 bytes) + tag_id (8 bytes)
  Value: timestamp (8 bytes)

Tree: file_meta
  Key: file_id (8 bytes) + key_len (2 bytes) + key (UTF-8)
  Value: UTF-8 string

Tree: settings
  Key: UTF-8 string
  Value: UTF-8 string

Tree: _meta
  Key: "version" | "backend" | "created_at"
  Value: varies
```

---

## Common Queries

### List directory contents

```rust
// Find all files where parent_id = directory_id
fn list_directory(storage: &dyn StorageBackend, dir_id: u64) -> Result<Vec<FileEntry>> {
    // Scan files collection, filter by parent_id
    // (SQLite can use index, KV stores scan and filter)
}
```

**SQLite:**

```sql
SELECT * FROM files WHERE parent_id = ? ORDER BY file_type DESC, name;
```

### Get file by path

```rust
fn get_file_by_path(storage: &dyn StorageBackend, path: &str) -> Result<Option<FileEntry>> {
    // 1. Look up file_id in paths collection
    // 2. Get FileEntry from files collection
}
```

**SQLite:**

```sql
SELECT f.* FROM files f
JOIN paths p ON f.id = p.file_id
WHERE p.path = ?;
```

### Get file content

```rust
fn get_content(storage: &dyn StorageBackend, file_id: u64) -> Result<Vec<u8>> {
    // 1. Get FileEntry, extract content_hash
    // 2. Get content from contents collection
}
```

### Version history

```rust
fn get_versions(storage: &dyn StorageBackend, file_id: u64) -> Result<Vec<VersionEntry>> {
    // Scan versions with prefix = file_id bytes
}
```

**SQLite:**

```sql
SELECT * FROM versions WHERE file_id = ? ORDER BY version_num DESC;
```

### Files by tag

```rust
fn files_with_tag(storage: &dyn StorageBackend, tag_name: &str) -> Result<Vec<FileEntry>> {
    // 1. Get tag_id from tag_names
    // 2. Scan file_tags with tag_id suffix
    // 3. Get FileEntry for each file_id
}
```

**SQLite:**

```sql
SELECT f.* FROM files f
JOIN file_tags ft ON f.id = ft.file_id
JOIN tags t ON ft.tag_id = t.id
WHERE t.name = ?;
```

---

## Full-Text Search

### SQLite (FTS5)

Built-in full-text search:

```sql
-- Search
SELECT path, snippet(fts_content, 1, '<b>', '</b>', '...', 32) as snippet
FROM fts_content
WHERE fts_content MATCH ?
ORDER BY rank;

-- Index on insert
INSERT INTO fts_content(rowid, path, content)
VALUES (?, ?, ?);

-- Update on content change
DELETE FROM fts_content WHERE rowid = ?;
INSERT INTO fts_content(rowid, path, content) VALUES (?, ?, ?);
```

### Tantivy (Sled/LMDB/RocksDB)

Separate search index using tantivy:

```rust
// Index structure
let schema = Schema::builder()
    .add_text_field("path", TEXT | STORED)
    .add_text_field("content", TEXT)
    .add_u64_field("file_id", INDEXED | STORED)
    .build();

// Search
let query_parser = QueryParser::for_index(&index, vec![content_field]);
let query = query_parser.parse_query(query_str)?;
let results = searcher.search(&query, &TopDocs::with_limit(limit))?;
```

---

## ID Generation

### Auto-increment (SQLite)

SQLite handles ID generation via `AUTOINCREMENT`.

### Monotonic IDs (KV stores)

For key-value backends, use a counter in the settings collection:

```rust
fn next_id(storage: &dyn StorageBackend, collection: &str) -> Result<u64> {
    storage.transaction(|txn| {
        let key = format!("_next_id_{}", collection);
        let current: u64 = txn.get("settings", key.as_bytes())?
            .map(|v| u64::from_be_bytes(v.try_into().unwrap()))
            .unwrap_or(0);
        let next = current + 1;
        txn.put("settings", key.as_bytes(), &next.to_be_bytes())?;
        Ok(next)
    })
}
```

---

## Maintenance Operations

### Garbage Collection

Remove unreferenced content blobs:

```rust
fn garbage_collect(storage: &dyn StorageBackend) -> Result<GcStats> {
    // 1. Scan contents_meta for ref_count = 0
    // 2. Delete from contents and contents_meta
}
```

**SQLite:**

```sql
DELETE FROM contents WHERE ref_count = 0;
```

### Version Pruning

Keep last N versions:

```rust
fn prune_versions(storage: &dyn StorageBackend, keep: u32) -> Result<PruneStats> {
    // For each file_id in versions:
    //   Get version count
    //   If > keep, delete oldest (version_num < max - keep)
    //   Decrement ref_count on deleted content hashes
}
```

**SQLite:**

```sql
DELETE FROM versions
WHERE (file_id, version_num) NOT IN (
    SELECT file_id, version_num FROM versions v2
    WHERE v2.file_id = versions.file_id
    ORDER BY version_num DESC
    LIMIT ?
);
```

### Compaction

Reclaim disk space:

| Backend | Method |
|---------|--------|
| SQLite | `VACUUM` |
| Sled | Automatic compaction |
| LMDB | Copy to new database |
| RocksDB | `compact_range()` |

---

## Migration

When migrating between backends, the migration tool:

1. Opens source with old backend
2. Creates destination with new backend
3. Iterates all collections, copying key-value pairs
4. Rebuilds search index
5. Verifies integrity (checksums)

Data format (bincode) is the same across backends, so no transformation is needed.
