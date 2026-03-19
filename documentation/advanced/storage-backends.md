# Storage Backends

VFS uses a pluggable storage backend architecture, allowing different embedded databases to be used as the underlying storage engine.

## Available Backends

| Backend | Best For | Trade-offs |
|---------|----------|------------|
| **SQLite** (default) | General use, queries | Larger files, slower writes |
| **Sled** | High write throughput | No SQL, newer |
| **LMDB** | Read-heavy, memory-mapped | Fixed map size |
| **RocksDB** | Large datasets, LSM | More dependencies |

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
avfs vault create myproject --backend sqlite
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
avfs vault create myproject --backend sled
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
avfs vault create myproject --backend lmdb
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
avfs vault create myproject --backend rocksdb
```

## Selecting a Backend

### At Vault Creation

```bash
# Explicit backend selection
avfs vault create myproject --backend sled

# Use default (SQLite)
avfs vault create myproject
```

### Choosing Based on Workload

| Workload | Recommended Backend |
|----------|-------------------|
| General purpose | SQLite |
| Write-heavy (logs, imports) | Sled or RocksDB |
| Read-heavy (serving) | LMDB |
| Large datasets (>10GB) | RocksDB |
| Maximum compatibility | SQLite |
| Minimal dependencies | Sled |

## Search Implementations

| Storage Backend | Search Implementation |
|-----------------|----------------------|
| SQLite | FTS5 (built-in) |
| Sled | Tantivy |
| LMDB | Tantivy |
| RocksDB | Tantivy |

## Migration Between Backends

Convert a vault from one backend to another:

```bash
avfs vault migrate myproject --to sled
```

This:

1. Creates a new vault with the target backend
2. Copies all data (files, versions, tags, metadata)
3. Verifies integrity
4. Optionally removes the old vault

### Migration Options

```bash
# Keep original vault
avfs vault migrate myproject --to sled --keep-original

# Custom destination
avfs vault migrate myproject --to lmdb --output /path/to/new.avfs

# Verify without migrating
avfs vault migrate myproject --to sled --dry-run
```

## Troubleshooting

### Backend Not Available

```
Error: Backend 'rocksdb' is not compiled in this build
```

VFS may be compiled without certain backends. Check available backends:

```bash
avfs --version
```

### Migration Fails

Try manual migration:

```bash
# Export all data
avfs export / /tmp/vault-export --recursive

# Create new vault with desired backend
avfs vault create newvault --backend sled

# Import data
avfs --vault newvault import /tmp/vault-export /
```

### Performance Issues

Check backend statistics:

```bash
avfs stats
```

Try compaction:

```bash
avfs compact
```
