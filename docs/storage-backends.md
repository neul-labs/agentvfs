# Storage Backends

vfs uses a pluggable storage backend architecture, allowing different embedded databases to be used as the underlying storage engine.

## Overview

The storage backend abstraction provides:
- **Flexibility**: Choose the best database for your use case
- **Portability**: Switch backends without changing vault data (via migration)
- **Extensibility**: Add new backends by implementing traits
- **Testing**: Use in-memory backends for fast tests

## Available Backends

All vault files use the `.vfs` extension regardless of backend. The backend type is stored in vault metadata.

| Backend | Best For | Trade-offs |
|---------|----------|------------|
| SQLite (default) | General use, queries | Larger files, slower writes |
| Sled | High write throughput | No SQL, newer |
| LMDB | Read-heavy, memory-mapped | Fixed map size |
| RocksDB | Large datasets, LSM | More dependencies |
| In-Memory | Testing | No persistence |

### SQLite (Default)

The default backend using SQLite with FTS5 for full-text search.

**Pros:**
- Battle-tested, widely used
- Built-in FTS5 for search
- SQL queries for debugging
- Single-file database
- Good tooling (DB Browser, etc.)

**Cons:**
- Write amplification for small updates
- Larger file size overhead
- Global write lock

**Usage:**
```bash
vfs vault create myproject --backend sqlite
```

**Configuration:**
```toml
[backend.sqlite]
journal_mode = "wal"      # wal, delete, truncate, memory
synchronous = "normal"    # off, normal, full, extra
cache_size = 10000        # pages (default page = 4KB)
mmap_size = 268435456     # 256MB memory-mapped I/O
```

### Sled

A modern embedded database written in Rust, using a Bw-tree architecture.

**Pros:**
- Pure Rust, no C dependencies
- High write throughput
- Lock-free reads
- Built-in compression

**Cons:**
- No SQL queries
- Requires separate search index (tantivy)
- Younger project

**Usage:**
```bash
vfs vault create myproject --backend sled
```

**Configuration:**
```toml
[backend.sled]
cache_capacity = 1073741824   # 1GB cache
flush_every_ms = 500          # Flush interval
compression = true            # Zstd compression
```

### LMDB

Lightning Memory-Mapped Database - extremely fast reads via memory mapping.

**Pros:**
- Very fast reads
- Memory-mapped for efficiency
- Proven in production (OpenLDAP)
- ACID compliant

**Cons:**
- Fixed maximum database size
- Write-heavy workloads less efficient
- Requires separate search index

**Usage:**
```bash
vfs vault create myproject --backend lmdb
```

**Configuration:**
```toml
[backend.lmdb]
map_size = 10737418240    # 10GB max database size
max_readers = 126         # Concurrent readers
no_sync = false           # Disable fsync (faster, less safe)
```

### RocksDB

LSM-tree based storage engine from Facebook, designed for SSDs.

**Pros:**
- Excellent for large datasets
- Good write performance
- Compression tiers
- Column families

**Cons:**
- Large binary dependency
- Complex tuning
- More resource usage

**Usage:**
```bash
vfs vault create myproject --backend rocksdb
```

**Configuration:**
```toml
[backend.rocksdb]
create_if_missing = true
max_open_files = 1000
write_buffer_size = 67108864    # 64MB
compression = "lz4"             # none, snappy, lz4, zstd
```

### In-Memory

Non-persistent backend for testing and temporary use.

**Usage:**
```bash
vfs vault create temp --backend memory

# Or for testing
VFS_BACKEND=memory cargo test
```

## Selecting a Backend

### At Vault Creation

```bash
# Explicit backend selection
vfs vault create myproject --backend sled

# Use default (SQLite)
vfs vault create myproject
```

### Default Backend

Set the default backend in global config:

```toml
# ~/.vfs/config.toml
[defaults]
backend = "sqlite"   # or sled, lmdb, rocksdb
```

### Per-Vault Detection

The backend type is stored in vault metadata. vfs automatically detects and uses the correct backend when opening a vault.

## Backend Trait Interface

All backends implement the `StorageBackend` trait:

```rust
pub trait StorageBackend: Send + Sync {
    /// Open or create a database at the given path
    fn open(path: &Path, options: &BackendOptions) -> Result<Self>
    where
        Self: Sized;

    /// Get a value by key from a collection
    fn get(&self, collection: &str, key: &[u8]) -> Result<Option<Vec<u8>>>;

    /// Store a key-value pair
    fn put(&self, collection: &str, key: &[u8], value: &[u8]) -> Result<()>;

    /// Delete a key
    fn delete(&self, collection: &str, key: &[u8]) -> Result<()>;

    /// Check if a key exists
    fn exists(&self, collection: &str, key: &[u8]) -> Result<bool>;

    /// Iterate over keys with a prefix
    fn scan_prefix(
        &self,
        collection: &str,
        prefix: &[u8],
    ) -> Result<Box<dyn Iterator<Item = Result<(Vec<u8>, Vec<u8>)>> + '_>>;

    /// Iterate over a key range
    fn scan_range(
        &self,
        collection: &str,
        start: &[u8],
        end: &[u8],
    ) -> Result<Box<dyn Iterator<Item = Result<(Vec<u8>, Vec<u8>)>> + '_>>;

    /// Execute a transaction
    fn transaction<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&mut dyn TransactionOps) -> Result<T>;

    /// Ensure all writes are persisted
    fn sync(&self) -> Result<()>;

    /// Optimize storage (compact, vacuum, etc.)
    fn compact(&self) -> Result<CompactionStats>;

    /// Get storage statistics
    fn stats(&self) -> Result<StorageStats>;

    /// Close the database
    fn close(self) -> Result<()>;
}

pub trait TransactionOps {
    fn get(&self, collection: &str, key: &[u8]) -> Result<Option<Vec<u8>>>;
    fn put(&mut self, collection: &str, key: &[u8], value: &[u8]) -> Result<()>;
    fn delete(&mut self, collection: &str, key: &[u8]) -> Result<()>;
}
```

## Search Backend Trait

Full-text search is a separate trait since not all databases have built-in FTS:

```rust
pub trait SearchBackend: Send + Sync {
    /// Index a document
    fn index(&self, doc_id: &str, fields: &[(&str, &str)]) -> Result<()>;

    /// Remove a document from the index
    fn remove(&self, doc_id: &str) -> Result<()>;

    /// Search for documents
    fn search(&self, query: &str, options: &SearchOptions) -> Result<SearchResults>;

    /// Rebuild the entire index
    fn rebuild(&self) -> Result<()>;

    /// Optimize the index
    fn optimize(&self) -> Result<()>;
}

pub struct SearchOptions {
    pub limit: usize,
    pub offset: usize,
    pub fields: Option<Vec<String>>,  // Fields to search
    pub highlight: bool,
}

pub struct SearchResults {
    pub hits: Vec<SearchHit>,
    pub total: usize,
    pub took_ms: u64,
}
```

### Search Implementations

| Storage Backend | Search Implementation |
|-----------------|----------------------|
| SQLite | FTS5 (built-in) |
| Sled | Tantivy |
| LMDB | Tantivy |
| RocksDB | Tantivy |
| Memory | Simple in-memory index |

## Implementing a New Backend

### Step 1: Create Backend Struct

```rust
pub struct MyBackend {
    db: MyDbHandle,
    path: PathBuf,
}

impl MyBackend {
    pub fn open(path: &Path, options: &BackendOptions) -> Result<Self> {
        let db = MyDbHandle::open(path)?;
        Ok(Self {
            db,
            path: path.to_path_buf(),
        })
    }
}
```

### Step 2: Implement StorageBackend

```rust
impl StorageBackend for MyBackend {
    fn get(&self, collection: &str, key: &[u8]) -> Result<Option<Vec<u8>>> {
        let full_key = make_key(collection, key);
        self.db.get(&full_key).map_err(Into::into)
    }

    fn put(&self, collection: &str, key: &[u8], value: &[u8]) -> Result<()> {
        let full_key = make_key(collection, key);
        self.db.put(&full_key, value).map_err(Into::into)
    }

    fn transaction<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&mut dyn TransactionOps) -> Result<T>,
    {
        let txn = self.db.begin_transaction()?;
        let mut ops = MyTransactionOps::new(txn);
        let result = f(&mut ops)?;
        ops.commit()?;
        Ok(result)
    }

    // ... implement remaining methods
}
```

### Step 3: Implement SearchBackend (or use Tantivy adapter)

```rust
// Option 1: Custom implementation
impl SearchBackend for MyBackend {
    fn search(&self, query: &str, options: &SearchOptions) -> Result<SearchResults> {
        // Your search implementation
    }
}

// Option 2: Use Tantivy adapter
pub struct MyBackendWithSearch {
    storage: MyBackend,
    search: TantivySearchBackend,
}
```

### Step 4: Register the Backend

```rust
// In src/backends/mod.rs
pub fn create_backend(
    backend_type: &str,
    path: &Path,
    options: &BackendOptions,
) -> Result<Box<dyn StorageBackend>> {
    match backend_type {
        "sqlite" => Ok(Box::new(SqliteBackend::open(path, options)?)),
        "sled" => Ok(Box::new(SledBackend::open(path, options)?)),
        "lmdb" => Ok(Box::new(LmdbBackend::open(path, options)?)),
        "mybackend" => Ok(Box::new(MyBackend::open(path, options)?)),
        _ => Err(VfsError::UnknownBackend(backend_type.to_string())),
    }
}
```

## Migration Between Backends

Convert a vault from one backend to another:

```bash
vfs vault migrate myproject --to sled
```

This:
1. Creates a new vault with the target backend
2. Copies all data (files, versions, tags, metadata)
3. Verifies integrity
4. Optionally removes the old vault

### Migration Options

```bash
# Keep original vault
vfs vault migrate myproject --to sled --keep-original

# Custom destination
vfs vault migrate myproject --to lmdb --output /path/to/new.vfs

# Verify without migrating
vfs vault migrate myproject --to sled --dry-run
```

## Performance Comparison

Benchmark results (example, actual results vary by hardware):

| Operation | SQLite | Sled | LMDB | RocksDB |
|-----------|--------|------|------|---------|
| Sequential writes (ops/s) | 50K | 200K | 100K | 300K |
| Random reads (ops/s) | 500K | 800K | 1.2M | 600K |
| Range scan (MB/s) | 150 | 200 | 400 | 250 |
| Full-text search (ms) | 5 | 8* | 8* | 8* |
| Database size (1M files) | 2.1GB | 1.8GB | 1.5GB | 1.6GB |

*Using Tantivy for search

### Choosing Based on Workload

| Workload | Recommended Backend |
|----------|-------------------|
| General purpose | SQLite |
| Write-heavy (logs, imports) | Sled or RocksDB |
| Read-heavy (serving) | LMDB |
| Large datasets (>10GB) | RocksDB |
| Maximum compatibility | SQLite |
| Minimal dependencies | Sled |

## Troubleshooting

### Backend Not Available

```
Error: Backend 'rocksdb' is not compiled in this build
```

vfs may be compiled without certain backends. Check available backends:

```bash
vfs --version --backends
```

Compile with specific backends:

```bash
cargo build --features "sqlite,sled,lmdb"
```

### Migration Fails

```
Error: Migration failed: integrity check failed
```

Try manual migration:

```bash
# Export all data
vfs export / /tmp/vault-export --recursive

# Create new vault with desired backend
vfs vault create newvault --backend sled

# Import data
vfs --vault newvault import /tmp/vault-export /
```

### Performance Issues

Check backend statistics:

```bash
vfs vault stats --backend-info
```

Try compaction:

```bash
vfs compact
```
