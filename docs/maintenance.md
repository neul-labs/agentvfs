# Maintenance & Pruning

vfs provides tools to manage vault size and performance through pruning old versions, garbage collection, and database compaction.

## Overview

Over time, vaults accumulate:
- Old file versions
- Orphaned content blobs (unreferenced data)
- Database fragmentation

Regular maintenance keeps vaults performant and storage-efficient.

## Pruning Strategies

vfs supports multiple pruning strategies that can be configured per vault.

### Keep N Versions

Retain only the last N versions of each file.

```bash
vfs prune --keep <N>
```

**Example:**

```bash
$ vfs prune --keep 5
Pruning versions, keeping last 5 per file...
Removed 1,234 old versions
Freed 45.2 MB (run 'vfs compact' to reclaim space)
```

### Time-Based Pruning

Remove versions older than N days.

```bash
vfs prune --older-than <DAYS>
```

**Example:**

```bash
$ vfs prune --older-than 30
Pruning versions older than 30 days...
Removed 892 old versions
Freed 23.1 MB
```

### Size-Based Pruning

Remove old versions until the vault is under a size limit.

```bash
vfs prune --max-size <MB>
```

**Example:**

```bash
$ vfs prune --max-size 500
Pruning to keep vault under 500 MB...
Current size: 752 MB
Removed 3,456 versions
New size: 489 MB
```

Size-based pruning removes oldest versions first while preserving at least one version per file.

### Combined Strategies

Strategies can be combined:

```bash
# Keep last 10 versions AND remove anything older than 90 days
vfs prune --keep 10 --older-than 90
```

### Prune Specific Files

Target specific files or patterns:

```bash
vfs prune --keep 3 /logs/*.log
vfs prune --older-than 7 /cache/
```

## Dry Run

Preview what would be pruned without actually deleting:

```bash
$ vfs prune --keep 5 --dry-run
DRY RUN - No changes will be made

Would remove:
  /docs/readme.txt: 12 versions (keeping 5)
  /src/main.rs: 8 versions (keeping 5)
  /config/settings.json: 3 versions (keeping all)
  ...

Total: 1,234 versions, 45.2 MB
```

## Vault Configuration

Set default pruning behavior for a vault:

```bash
# Set strategy
vfs vault config prune_strategy keep_n

# Set parameters
vfs vault config prune_keep_count 10
vfs vault config prune_max_age_days 30
vfs vault config prune_max_size_mb 1000
```

### Auto-Pruning

Enable automatic pruning on every write:

```bash
vfs vault config auto_prune true
```

With auto-pruning, old versions are cleaned up automatically based on the configured strategy.

## Garbage Collection

Garbage collection removes orphaned content blobs - data that's no longer referenced by any file or version.

### When Orphans Occur

Content becomes orphaned when:
- All versions referencing it are pruned
- Files are deleted
- Content is replaced

### Run GC

```bash
vfs gc
```

**Example:**

```bash
$ vfs gc
Scanning for orphaned content...
Found 234 orphaned blobs (12.5 MB)
Removing orphaned content...
Done. Run 'vfs compact' to reclaim disk space.
```

### Dry Run

```bash
$ vfs gc --dry-run
DRY RUN - No changes will be made

Would remove 234 orphaned blobs (12.5 MB)
```

### GC Statistics

```bash
$ vfs gc --stats
Content blobs: 5,678
Referenced: 5,444
Orphaned: 234 (12.5 MB)
```

## Compaction

After pruning and GC, the database file still occupies the same disk space. Compaction reclaims unused space.

```bash
vfs compact
```

**Example:**

```bash
$ vfs compact
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

### When to Compact

- After large pruning operations
- After deleting many files
- When vault size seems larger than expected
- Periodically (weekly/monthly for active vaults)

## Full Maintenance Routine

Complete maintenance in one command:

```bash
vfs maintain
```

This runs:
1. Prune (using vault's configured strategy)
2. Garbage collection
3. Compaction
4. Index optimization

**Example:**

```bash
$ vfs maintain
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

### Scheduled Maintenance

For regular maintenance, set up a cron job:

```bash
# Run weekly maintenance on all vaults
0 3 * * 0 vfs vault list --names | xargs -I {} vfs --vault {} maintain
```

## Monitoring Vault Health

### Vault Statistics

```bash
vfs vault info
```

Shows current vault state including:
- Total size
- File count
- Version count
- Content blob count
- Index size

### Storage Breakdown

```bash
$ vfs vault stats
Storage Breakdown:
  Content blobs:  89.2 MB (1,234 blobs)
  Version data:   12.3 MB (5,678 versions)
  Metadata:       2.1 MB
  FTS index:      8.5 MB
  Overhead:       3.2 MB
  ──────────────────────
  Total:          115.3 MB
```

### Version Distribution

```bash
$ vfs vault stats --versions
Files by version count:
  1-5 versions:    234 files
  6-10 versions:   156 files
  11-20 versions:  89 files
  21-50 versions:  45 files
  50+ versions:    12 files

Top files by version count:
  /logs/app.log        156 versions
  /config/settings.json 89 versions
  /data/cache.json      72 versions
```

## Troubleshooting

### Vault Size Not Decreasing

After pruning/GC, run compact:

```bash
vfs compact
```

If size still doesn't decrease, check for:
- Large remaining files: `vfs find / -size +10M`
- Files with many versions: `vfs vault stats --versions`

### Slow Pruning

For large vaults, pruning can be slow. Use batching:

```bash
# Prune in batches by directory
for dir in /docs /src /data; do
    vfs prune --keep 5 "$dir"
done
```

### Compact Fails

If VACUUM fails due to disk space:

```bash
# Set custom temp directory with more space
TMPDIR=/path/with/space vfs compact
```

Or use incremental vacuum:

```bash
sqlite3 ~/.vfs/vaults/vault.vfs "PRAGMA incremental_vacuum(1000)"
```

### Recover from Aggressive Pruning

If you pruned too aggressively:
- Recent changes may still be in SQLite WAL
- Check for backups
- For critical data, consider professional SQLite recovery

Prevention: Always use `--dry-run` first.

## Best Practices

1. **Regular maintenance**: Run `vfs maintain` weekly for active vaults
2. **Dry run first**: Always preview with `--dry-run` before pruning
3. **Backup before major cleanup**: Export important files before aggressive pruning
4. **Set reasonable defaults**: Configure vault with sensible prune settings
5. **Monitor growth**: Check `vfs vault info` periodically
6. **Compact after bulk operations**: Always compact after large imports/deletes
