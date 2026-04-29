# Maintenance

VFS provides tools to manage vault size and performance through pruning old versions, garbage collection, and database compaction.

## Overview

Over time, vaults accumulate:

- Old file versions
- Orphaned content blobs (unreferenced data)
- Database fragmentation

Regular maintenance keeps vaults performant and storage-efficient.

## Pruning Strategies

### Keep N Versions

Retain only the last N versions of each file:

```bash
avfs prune --keep <N>
```

**Example:**

```bash
$ avfs prune --keep 5
Pruning versions, keeping last 5 per file...
Removed 1,234 old versions
Freed 45.2 MB (run 'avfs compact' to reclaim space)
```

### Time-Based Pruning

Remove versions older than N days:

```bash
avfs prune --older-than <DAYS>
```

**Example:**

```bash
$ avfs prune --older-than 30
Pruning versions older than 30 days...
Removed 892 old versions
Freed 23.1 MB
```

### Combined Strategies

Strategies can be combined:

```bash
# Keep last 10 versions AND remove anything older than 90 days
avfs prune --keep 10 --older-than 90
```

### Prune Specific Files

Target specific files or patterns:

```bash
avfs prune --keep 3 /logs/*.log
avfs prune --older-than 7 /cache/
```

## Dry Run

Preview what would be pruned without actually deleting:

```bash
$ avfs prune --keep 5 --dry-run
DRY RUN - No changes will be made

Would remove:
  /docs/readme.txt: 12 versions (keeping 5)
  /src/main.rs: 8 versions (keeping 5)
  ...

Total: 1,234 versions, 45.2 MB
```

## Garbage Collection

Garbage collection removes orphaned content blobs - data that's no longer referenced by any file or version.

### When Orphans Occur

Content becomes orphaned when:

- All versions referencing it are pruned
- Files are deleted
- Content is replaced

### Run GC

```bash
avfs gc
```

**Example:**

```bash
$ avfs gc
Scanning for orphaned content...
Found 234 orphaned blobs (12.5 MB)
Removing orphaned content...
Done. Run 'avfs compact' to reclaim disk space.
```

## Compaction

After pruning and GC, the database file still occupies the same disk space. Compaction reclaims unused space.

```bash
avfs compact
```

**Example:**

```bash
$ avfs compact
Compacting vault 'myproject'...
Before: 125.3 MB
After: 89.7 MB
Freed: 35.6 MB
```

### What Compact Does

1. Runs garbage collection (if not recently run)
2. Rebuilds FTS index
3. Executes SQLite VACUUM
4. Optimizes indexes

## Full Maintenance Routine

Complete maintenance in one command:

```bash
avfs maintain
```

This runs:

1. Prune (using vault's configured strategy)
2. Garbage collection
3. Compaction
4. Index optimization

**Example:**

```bash
$ avfs maintain
Starting maintenance for vault 'myproject'...

[1/4] Pruning old versions...
      Removed 456 versions

[2/4] Garbage collection...
      Removed 123 orphaned blobs

[3/4] Compacting database...
      Freed 28.4 MB

[4/4] Optimizing indexes...
      Done

Maintenance complete.
Before: 156.2 MB
After: 127.8 MB
```

## Monitoring Vault Health

### Vault Statistics

```bash
avfs stats
```

Shows:

- Total size
- File count
- Version count
- Content blob count

### Storage Breakdown

```bash
$ avfs stats
Storage Breakdown:
  Content blobs:  89.2 MB (1,234 blobs)
  Version data:   12.3 MB (5,678 versions)
  Metadata:       2.1 MB
  FTS index:      8.5 MB
  ──────────────────────
  Total:          115.3 MB
```

## Best Practices

1. **Regular maintenance**: Run `avfs maintain` weekly for active vaults
2. **Dry run first**: Always preview with `--dry-run` before pruning
3. **Backup before major cleanup**: Export important files before aggressive pruning
4. **Compact after bulk operations**: Always compact after large imports/deletes

## Troubleshooting

### Vault Size Not Decreasing

After pruning/GC, run compact:

```bash
avfs compact
```

If size still doesn't decrease, check for:

- Large remaining files: `avfs find / --min-size 10485760`
- Files with many versions: `avfs stats`

### Slow Pruning

For large vaults, prune in batches by directory:

```bash
for dir in /docs /src /data; do
    avfs prune --keep 5 "$dir"
done
```

### Recover from Aggressive Pruning

If you pruned too aggressively:

- Check for backups
- Recent changes may still be in SQLite WAL

Prevention: Always use `--dry-run` first.
