# Core Concepts

Understanding these concepts will help you get the most out of VFS.

## Vaults

A **vault** is a self-contained virtual filesystem stored in a single database file.

```
~/.vfs/
├── project-a.vfs    # One vault
├── project-b.vfs    # Another vault
└── experiments.vfs  # A third vault
```

### Key Properties

- **Isolated**: Each vault is completely independent
- **Portable**: Copy the `.vfs` file to move your entire filesystem
- **Single File**: Everything (files, directories, metadata, versions) in one file

### Working with Vaults

```bash
# Create a vault
vfs vault create myproject

# List vaults (current marked with *)
vfs vault list

# Switch vaults
vfs vault use another-project

# Use a specific vault for one command
vfs --vault myproject ls /
```

## Content-Addressable Storage

VFS uses **content-addressable storage** (CAS), where files are stored by the SHA-256 hash of their content.

```
                    ┌─────────────────┐
    file-a.txt ───▶ │  sha256: abc123 │
                    │  "Hello World"  │
    file-b.txt ───▶ │                 │ ◀─── Same content,
                    └─────────────────┘      stored once!
```

### Benefits

1. **Automatic Deduplication**: Identical content is stored only once
2. **Efficient Versioning**: New versions only store changed content
3. **Integrity**: Content is verified by hash on every read
4. **Space Savings**: Multiple files pointing to same content share storage

### Example

```bash
# Create two files with identical content
vfs write /file1.txt "Hello World"
vfs write /file2.txt "Hello World"

# Check storage - only one blob exists!
vfs stats
# Blobs: 1, Total size: 11 bytes
```

## Automatic Versioning

Every modification to a file creates a new **version**. VFS keeps the complete history.

```
/docs/readme.txt
├── Version 1: "Initial content"     (created)
├── Version 2: "Updated content"     (modified)
├── Version 3: "Fixed typo"          (modified)
└── Version 4: "Final version"       (current)
```

### How It Works

1. **Write Operation**: Creates new version, old versions preserved
2. **Version Storage**: Each version points to a content blob (via SHA-256)
3. **No Deltas**: Full content stored (but deduplicated via CAS)

### Working with Versions

```bash
# View history
vfs log /docs/readme.txt

# Read specific version
vfs cat -v 2 /docs/readme.txt

# Compare versions
vfs diff /docs/readme.txt --v1 1 --v2 3

# Restore old version (creates new version with old content)
vfs checkout /docs/readme.txt -v 2
```

## File Entries vs Content Blobs

VFS separates **metadata** (file entries) from **content** (blobs).

```
┌─────────────────────────────────┐
│         File Entry              │
├─────────────────────────────────┤
│ id: 42                          │
│ path: /docs/readme.txt          │
│ type: file                      │
│ size: 1024                      │
│ content_hash: sha256:abc123...  │──▶ ┌──────────────┐
│ created: 2024-01-15 10:00       │    │ Content Blob │
│ modified: 2024-01-15 12:30      │    │ "Hello..."   │
│ version: 3                      │    └──────────────┘
└─────────────────────────────────┘
```

### File Entry Contains

- Path and name
- Type (file or directory)
- Size
- Timestamps (created, modified)
- Version number
- Reference to content blob
- Parent directory reference

### Content Blob Contains

- Raw file content
- Identified by SHA-256 hash
- Shared across files/versions with same content

## Tags and Metadata

VFS supports rich metadata on files.

### Tags

Tags are simple labels attached to files:

```bash
vfs tag /report.pdf urgent
vfs tag /report.pdf quarterly
vfs find --tag urgent
```

### Custom Metadata

Key-value pairs for structured data:

```bash
vfs meta /report.pdf author "Jane Doe"
vfs meta /report.pdf department "Finance"
vfs meta /report.pdf status "approved"
```

### Metadata Use Cases

- **Document Management**: Author, status, review date
- **Asset Tracking**: License, source, expiration
- **Workflow**: Priority, assigned-to, due-date

## Storage Backends

VFS supports multiple database backends:

| Backend | Best For | Trade-offs |
|---------|----------|------------|
| **SQLite** | General use | Excellent tooling, FTS5 search |
| **Sled** | High write throughput | Pure Rust, modern |
| **LMDB** | Read-heavy workloads | Memory-mapped, fast reads |
| **RocksDB** | Large datasets | LSM-tree, SSD-optimized |

The backend affects:

- Performance characteristics
- Search implementation (FTS5 vs Tantivy)
- File size limits
- Concurrency behavior

See [Storage Backends](../advanced/storage-backends.md) for details.

## The Root Directory

Every vault has a root directory `/`. This is always your starting point.

```bash
# Root is always accessible
vfs ls /

# All paths are absolute from root
vfs cat /docs/readme.txt

# There is no "current directory" in VFS
# (except in interactive shell mode)
```

!!! note "No Relative Paths"
    In normal CLI usage, all paths are absolute. The interactive shell
    supports `cd` and relative paths for convenience.

## Snapshots

Snapshots capture the entire vault state at a point in time.

```bash
# Save current state
vfs snapshot save before-refactor

# Make changes...
vfs rm -r /old-code
vfs mv /new-code /production

# Oops! Restore previous state
vfs snapshot restore before-refactor
```

Snapshots are useful for:

- **Experimentation**: Try changes safely
- **Checkpoints**: Save state before major operations
- **AI Agents**: Provide rollback capability

## Next Steps

Now that you understand the concepts:

- [File Operations](../user-guide/file-operations.md) - Master file management
- [Versioning](../user-guide/versioning.md) - Deep dive into version control
- [Search](../user-guide/search.md) - Find anything quickly
