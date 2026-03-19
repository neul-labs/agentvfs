# Architecture Overview

## Design Philosophy

VFS is designed around these core principles:

1. **Familiarity**: Commands mirror standard Unix utilities
2. **Isolation**: Virtual filesystems are completely separate from the host filesystem
3. **Portability**: Each vault is a single database file
4. **Integrity**: All operations are transactional - no partial writes
5. **Efficiency**: Content-addressable storage for automatic deduplication
6. **Extensibility**: Pluggable storage backends allow different database engines

---

## System Architecture

```
+-------------------------------------------------------------+
|                        CLI Layer                             |
|  +-----+ +-----+ +-----+ +-----+ +-----+ +-----+ +-----+    |
|  | ls  | | cp  | | cat | |grep | | tag | |exec | |shell|    |
|  +--+--+ +--+--+ +--+--+ +--+--+ +--+--+ +--+--+ +--+--+    |
+----+-------+-------+-------+-------+-------+-------+---------+
     |       |       |       |       |       |       |
+----v-------v-------v-------v-------v-------v-------v---------+
|                     Command Layer                            |
|  +--------------+ +--------------+ +----------------------+  |
|  | FileCommands | |SearchCommands| |  MaintenanceCommands |  |
|  +------+-------+ +------+-------+ +----------+-----------+  |
+---------+----------------+--------------------+---------------+
          |                |                    |
+---------v----------------v--------------------v---------------+
|                     Storage Layer                            |
|  +-------------+ +-------------+ +-------------------------+ |
|  |PathResolver | |ContentStore | |    VersionManager       | |
|  +-------------+ +-------------+ +-------------------------+ |
+-----------------------------+---------------------------------+
                              |
+-----------------------------v---------------------------------+
|                  Storage Backend Trait                        |
|  +--------------------------------------------------------+  |
|  |  trait StorageBackend {                                |  |
|  |      fn get(&self, key) -> Result<Value>               |  |
|  |      fn put(&mut self, key, value) -> Result<()>       |  |
|  |      fn delete(&mut self, key) -> Result<()>           |  |
|  |      fn scan(&self, prefix) -> Result<Iterator>        |  |
|  |      fn transaction<F>(&mut self, f: F) -> Result<T>   |  |
|  |  }                                                     |  |
|  +--------------------------------------------------------+  |
+------------+------------------+------------------+-------------+
             |                  |                  |
     +-------v-------+  +-------v-------+  +-------v-------+
     |    SQLite     |  |     Sled      |  |     LMDB      |
     |    Backend    |  |    Backend    |  |    Backend    |
     +---------------+  +---------------+  +---------------+
```

---

## Components

### CLI Layer

Parses command-line arguments using `clap`. Each command is a subcommand with its own argument structure. The interactive shell (`vfs shell`) provides a REPL that dispatches to the same command handlers.

### Command Layer

Business logic for each operation. Commands are stateless functions that receive parsed arguments and a storage interface. Each command:

- Validates inputs
- Performs the operation within a transaction
- Returns results or errors

Commands are **backend-agnostic** - they interact only with the Storage Layer traits, never with database-specific APIs.

### Storage Layer

The storage layer provides high-level abstractions over the raw database:

#### PathResolver

- Resolves virtual paths to file IDs
- Handles path normalization (`/foo/../bar` -> `/bar`)
- Manages current working directory state
- Validates path existence and permissions

#### ContentStore

- Content-addressable storage using SHA-256 hashes
- Deduplicates identical content across files
- Handles BLOB storage and retrieval
- Manages search indexes (backend-specific)

#### VersionManager

- Creates new versions on every file modification
- Stores version metadata (timestamp, size, hash)
- Handles version retrieval and restoration
- Implements pruning strategies

### Storage Backend Trait

The `StorageBackend` trait defines the interface that all database backends must implement:

```rust
pub trait StorageBackend: Send + Sync {
    /// Get a value by key
    fn get(&self, collection: &str, key: &[u8]) -> Result<Option<Vec<u8>>>;

    /// Store a value
    fn put(&self, collection: &str, key: &[u8], value: &[u8]) -> Result<()>;

    /// Delete a value
    fn delete(&self, collection: &str, key: &[u8]) -> Result<()>;

    /// Check if key exists
    fn exists(&self, collection: &str, key: &[u8]) -> Result<bool>;

    /// Scan keys with prefix
    fn scan_prefix(&self, collection: &str, prefix: &[u8])
        -> Result<Box<dyn Iterator<Item = (Vec<u8>, Vec<u8>)>>>;

    /// Execute operations atomically
    fn transaction<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&dyn TransactionContext) -> Result<T>;

    /// Flush to disk
    fn sync(&self) -> Result<()>;

    /// Compact/optimize storage
    fn compact(&self) -> Result<()>;
}

pub trait SearchBackend: Send + Sync {
    /// Index content for full-text search
    fn index(&self, id: &str, content: &str) -> Result<()>;

    /// Remove from index
    fn unindex(&self, id: &str) -> Result<()>;

    /// Full-text search
    fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>>;
}
```

See [Storage Backends](../advanced/storage-backends.md) for implementation details.

---

## Data Model

The storage layer uses these logical collections (mapped to tables/trees/buckets by backends):

| Collection | Key | Value |
|------------|-----|-------|
| `files` | file_id (u64) | FileMetadata (serialized) |
| `paths` | path (string) | file_id (u64) |
| `contents` | hash (SHA-256) | blob data |
| `versions` | file_id + version_num | VersionMetadata |
| `tags` | tag_name | tag_id |
| `file_tags` | file_id + tag_id | timestamp |
| `metadata` | file_id + key | value |
| `settings` | key | value |

Data is serialized using a compact binary format (e.g., bincode, MessagePack, or CBOR).

---

## Vault Concept

A **vault** is an independent virtual filesystem stored in a single database file. Users can:

- Create multiple vaults for different projects
- Switch between vaults with `vfs vault use`
- Back up vaults by copying the database file

Default vault location: `~/.vfs/vaults/`

```
~/.vfs/
+-- config.toml          # Global configuration
+-- current_vault        # Tracks active vault
+-- vaults/
    +-- default.vfs      # SQLite backend (or .sled, .lmdb)
    +-- myproject.vfs
    +-- experiments.vfs
```

Vault files use the `.vfs` extension regardless of backend, with the backend type stored in metadata.

---

## Content-Addressable Storage

Files are stored using content-addressable storage (CAS):

1. When a file is written, its content is hashed with SHA-256
2. The content is stored in the `contents` collection keyed by hash
3. The file entry references the content hash
4. Multiple files with identical content share the same blob

**Benefits:**

- **Deduplication**: Identical content stored once
- **Integrity**: Hash verification catches corruption
- **Efficient versioning**: Unchanged content isn't duplicated

```
files                          contents
+------------------------+     +------------------------------+
| path: /docs/readme.txt |---->| hash: abc123...              |
| content_hash: abc123...|     | data: "Hello, World!"        |
+------------------------+     | size: 13                     |
+------------------------+     | ref_count: 2                 |
| path: /backup/copy.txt |---->+------------------------------+
| content_hash: abc123...|
+------------------------+
```

---

## Transaction Safety

All storage backends must provide ACID transactions:

- **Atomic**: Operations either complete fully or not at all
- **Consistent**: Data integrity is maintained
- **Isolated**: Concurrent operations don't interfere
- **Durable**: Committed changes survive crashes

The `transaction()` method wraps multiple operations atomically.

---

## Search Architecture

Full-text search is handled by a separate `SearchBackend` trait:

| Backend | Search Implementation |
|---------|----------------------|
| SQLite | FTS5 extension (built-in) |
| Sled/LMDB | Tantivy (embedded Lucene-like engine) |
| External | Can integrate with MeiliSearch, Elasticsearch |

The search index is kept in sync with content changes through the ContentStore.

---

## Error Handling

Errors are propagated using Rust's `Result` type:

```rust
pub enum VfsError {
    NotFound(PathBuf),
    AlreadyExists(PathBuf),
    NotADirectory(PathBuf),
    NotAFile(PathBuf),
    InvalidPath(String),
    StorageError(Box<dyn std::error::Error>),
    SerializationError(String),
    IoError(std::io::Error),
}
```

Backend-specific errors are wrapped in `StorageError` to maintain abstraction.

---

## Adding a New Backend

To add a new storage backend:

1. Implement `StorageBackend` trait
2. Implement `SearchBackend` trait (or use tantivy adapter)
3. Register in the backend factory
4. Add CLI flag: `--backend <name>`

See [Storage Backends](../advanced/storage-backends.md) for implementation guide.
