# Versioning

avfs automatically tracks version history for all files. Every time a file is modified, a new version is created, allowing you to view history, compare changes, and restore previous versions.

## Automatic Versioning

Unlike traditional version control systems, avfs doesn't require explicit commits. Versions are created automatically:

- **Every write operation** creates a new version
- **No manual commits** needed
- **Instant rollback** to any previous state
- **Zero-effort history** for all files

### When Versions Are Created

| Operation | Creates Version? |
|-----------|-----------------|
| `avfs write` | Yes |
| `avfs cp` (overwrite) | Yes |
| `avfs mv` (overwrite) | Yes |
| `avfs exec` (modifies file) | Yes |
| Pipe write | Yes |
| `avfs touch` | No (metadata only) |
| `avfs rm` | No (file deleted) |

## Viewing History

### File Version Log

```bash
avfs log <path>
```

Shows version history for a file:

```bash
$ avfs log /docs/readme.txt
VERSION  SIZE     DATE                 HASH (first 8)
7        1.2 KB   2024-03-10 14:22:15  a1b2c3d4
6        1.1 KB   2024-03-09 10:15:00  e5f6g7h8
5        1.0 KB   2024-03-08 09:30:22  i9j0k1l2
4        980 B    2024-03-07 16:45:11  m3n4o5p6
3        850 B    2024-03-05 11:20:00  q7r8s9t0
2        500 B    2024-03-01 09:00:00  u1v2w3x4
1        128 B    2024-02-28 15:30:00  y5z6a7b8
```

### Compact View

```bash
$ avfs log --oneline /docs/readme.txt
7: 1.2KB 2024-03-10 a1b2c3d4
6: 1.1KB 2024-03-09 e5f6g7h8
5: 1.0KB 2024-03-08 i9j0k1l2
...
```

### Limit History

```bash
avfs log -n 5 /docs/readme.txt    # Last 5 versions only
```

### JSON Output

```bash
avfs log --json /docs/readme.txt
```

## Reading Past Versions

### Cat with Version

```bash
avfs cat -v <version> <path>
```

Read a specific version:

```bash
$ avfs cat -v 3 /docs/readme.txt
# This shows the content from version 3
```

### Export Past Version

```bash
avfs export --version 3 /docs/readme.txt ~/old-readme.txt
```

## Comparing Versions

### Diff Current vs Past

```bash
avfs diff -v <version> <path>
```

Compare current version with a past version:

```bash
$ avfs diff -v 5 /docs/readme.txt
--- /docs/readme.txt (version 5)
+++ /docs/readme.txt (current)
@@ -1,4 +1,6 @@
 # README

-This is the old content.
+This is the updated content.
+With new lines added.
+And more changes.
```

### Diff Two Past Versions

```bash
avfs diff -v 3 -v 5 /docs/readme.txt
```

Compare version 3 to version 5.

### Diff Options

```bash
avfs diff -v 3 --color /docs/readme.txt      # Colorized output
avfs diff -v 3 --unified 5 /docs/readme.txt  # 5 lines of context
avfs diff -v 3 --side-by-side /docs/readme.txt
```

## Restoring Versions

### Checkout

```bash
avfs checkout <path> <version>
```

Restore a file to a specific version. This creates a **new version** (non-destructive):

```bash
$ avfs checkout /docs/readme.txt 3
Restored /docs/readme.txt to version 3
(created as version 8)

$ avfs log -n 2 /docs/readme.txt
VERSION  SIZE     DATE                 HASH
8        850 B    2024-03-10 15:00:00  q7r8s9t0  # Same content as v3
7        1.2 KB   2024-03-10 14:22:15  a1b2c3d4
```

### Revert

```bash
avfs revert <path>
```

Shortcut to restore the immediately previous version:

```bash
$ avfs revert /docs/readme.txt
Reverted /docs/readme.txt from version 7 to version 6
(created as version 8)
```

### Revert Multiple Times

Each revert creates a new version, so you can chain reverts:

```bash
avfs revert /docs/readme.txt    # v7 → v6 (creates v8)
avfs revert /docs/readme.txt    # v8 → v7 (creates v9)
# Now content matches original v7
```

## Version Storage

### How Versions Work

avfs uses **content-addressable storage**:

1. File content is hashed (SHA-256)
2. Content is stored in `file_contents` table
3. Version metadata points to the content hash
4. Identical content is stored only once

```
Version 1: hash → abc123 (points to content "Hello")
Version 2: hash → def456 (points to content "Hello World")
Version 3: hash → abc123 (same as v1, no extra storage)
```

### Storage Efficiency

- Unchanged content isn't duplicated
- Only actual changes consume space
- Reference counting tracks content usage

### Version Metadata

Each version stores:
- Version number (sequential per file)
- Content hash
- Size
- Timestamp

## Directory Versioning

Directories themselves don't have versions, but you can track changes across a directory:

### Log for Directory

```bash
avfs log /docs/
```

Shows recent versions across all files in the directory:

```bash
$ avfs log /docs/
PATH                    VERSION  DATE
/docs/readme.txt        7        2024-03-10 14:22:15
/docs/guide.md          3        2024-03-10 12:00:00
/docs/api.txt           12       2024-03-09 16:30:00
/docs/readme.txt        6        2024-03-09 10:15:00
...
```

### Recursive Checkout

Restore an entire directory to a point in time:

```bash
avfs checkout --before "2024-03-01" /docs/
```

Restores all files to their state before the given date.

## Pruning Old Versions

To manage storage, you can prune old versions. See [maintenance.md](maintenance.md) for details.

Quick overview:

```bash
# Keep only last 5 versions per file
avfs prune --keep 5

# Remove versions older than 30 days
avfs prune --older-than 30

# Preview what would be removed
avfs prune --keep 5 --dry-run
```

## Best Practices

### Use Tags for Milestones

Mark important versions with tags:

```bash
# Make changes
avfs write /config/app.yaml "..."

# Tag the current state
avfs tag /config/app.yaml release-v1.0
```

Later, find tagged versions:

```bash
avfs log /config/app.yaml | grep release
```

### Export Before Major Changes

Before risky operations:

```bash
avfs export /important/file.txt ~/backup-$(date +%Y%m%d).txt
```

### Regular Pruning

Set up automatic pruning to manage storage:

```bash
# Configure vault for time-based pruning
avfs vault config prune_strategy time_based
avfs vault config prune_max_age_days 90

# Run pruning
avfs prune
```

## Troubleshooting

### "Version not found"

The version may have been pruned. Check available versions:

```bash
avfs log /path/to/file
```

### Large Version Count

If a file has too many versions:

```bash
# Check version count
avfs log /file | wc -l

# Prune aggressively
avfs prune --keep 10 /file
```

### Recover Deleted File

Deleting a file removes it from the filesystem but versions may still exist in the database until garbage collection:

```bash
# Versions are preserved for deleted files
# Restore by creating the file again
avfs write /deleted/file.txt "placeholder"
avfs checkout /deleted/file.txt 5  # Restore old version
```
