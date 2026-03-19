# Vault Management

A **vault** is an independent virtual filesystem stored in a single database file (`.avfs` extension). avfs supports multiple vaults, allowing you to organize different projects, experiments, or backups in isolated containers.

## Concepts

### What is a Vault?

- A complete virtual filesystem in one `.avfs` file
- Contains files, directories, versions, tags, and metadata
- Fully isolated from other vaults
- Portable - copy the file to move your filesystem

### Default Storage Location

```
~/.avfs/
├── config.toml          # Global avfs configuration
├── current              # Name of the active vault
└── vaults/
    ├── default.avfs       # The default vault
    ├── myproject.avfs     # User-created vault
    └── experiments.avfs   # Another vault
```

## Creating Vaults

### Basic Creation

```bash
avfs vault create <name>
```

Creates a new vault in the default location (`~/.avfs/vaults/<name>.avfs`).

```bash
$ avfs vault create myproject
Created vault 'myproject' at ~/.avfs/vaults/myproject.avfs
Switched to vault 'myproject'
```

### Custom Location

```bash
avfs vault create <name> --path <path>
```

Store the vault database at a custom location:

```bash
# External drive
avfs vault create backup --path /mnt/external/backup.avfs

# Project directory
avfs vault create project-files --path ./project.avfs

# Absolute path
avfs vault create archive --path /data/archives/2024.avfs
```

### Create Without Switching

```bash
avfs vault create experiments --no-switch
```

Creates the vault but keeps the current vault active.

## Listing Vaults

```bash
$ avfs vault list
  VAULT          SIZE      FILES    VERSIONS   PATH
* default        12.3 MB   234      1,892      ~/.avfs/vaults/default.avfs
  myproject      4.5 MB    89       456        ~/.avfs/vaults/myproject.avfs
  experiments    128 KB    12       24         ~/.avfs/vaults/experiments.avfs
  backup         2.1 GB    5,678    45,678     /mnt/external/backup.avfs

* = active vault
```

### JSON Output

```bash
avfs vault list --json
```

Useful for scripting:

```json
[
  {
    "name": "default",
    "path": "~/.avfs/vaults/default.avfs",
    "size_bytes": 12902400,
    "file_count": 234,
    "version_count": 1892,
    "active": true
  }
]
```

## Switching Vaults

```bash
avfs vault use <name>
```

Switch to a different vault:

```bash
$ avfs vault use myproject
Switched to vault 'myproject'
Current directory: /

$ avfs pwd
/
```

### Temporary Vault Override

Use `--vault` flag with any command to temporarily use a different vault:

```bash
# List files in another vault without switching
avfs --vault backup ls /documents

# Copy between vaults
avfs cat /file.txt | avfs --vault backup write /file.txt
```

## Vault Information

```bash
avfs vault info [name]
```

Show detailed information about a vault:

```bash
$ avfs vault info myproject
Vault: myproject
Path: ~/.avfs/vaults/myproject.avfs
Size: 4.5 MB

Statistics:
  Files: 89
  Directories: 23
  Total versions: 456
  Content blobs: 312

Settings:
  Prune strategy: keep_n
  Keep versions: 10
  Max age (days): 30
  Max size (MB): 1000

Created: 2024-01-15 10:30:00
Last modified: 2024-03-10 14:22:15
```

## Deleting Vaults

```bash
avfs vault delete <name>
```

Delete a vault permanently:

```bash
$ avfs vault delete experiments
This will permanently delete vault 'experiments' and all its contents.
Are you sure? [y/N] y
Deleted vault 'experiments'
```

### Force Delete

Skip confirmation prompt:

```bash
avfs vault delete experiments --force
```

### Cannot Delete Active Vault

```bash
$ avfs vault delete myproject
Error: Cannot delete the active vault.
Switch to another vault first: avfs vault use default
```

## Vault Configuration

Each vault has its own settings stored in the `settings` table.

### View Settings

```bash
avfs vault config
```

### Modify Settings

```bash
avfs vault config <key> <value>
```

**Available settings:**

| Key | Default | Description |
|-----|---------|-------------|
| `prune_strategy` | `keep_n` | Pruning strategy (`keep_n`, `time_based`, `size_based`) |
| `prune_keep_count` | `10` | Versions to keep (for `keep_n`) |
| `prune_max_age_days` | `30` | Max version age (for `time_based`) |
| `prune_max_size_mb` | `1000` | Max vault size (for `size_based`) |

```bash
# Keep 20 versions instead of 10
avfs vault config prune_keep_count 20

# Switch to time-based pruning
avfs vault config prune_strategy time_based
avfs vault config prune_max_age_days 90
```

## Backup and Restore

### Backup a Vault

Since a vault is a single SQLite file, backup is simple:

```bash
# Copy the database file
cp ~/.avfs/vaults/myproject.avfs ~/backups/myproject-$(date +%Y%m%d).avfs

# Or use avfs export for a tarball
avfs export / ~/backups/myproject-export.tar.gz --recursive
```

### Online Backup

For backing up while avfs is in use:

```bash
sqlite3 ~/.avfs/vaults/myproject.avfs ".backup ~/backups/myproject.avfs"
```

### Restore from Backup

```bash
# Register the backup as a vault
avfs vault create restored --path ~/backups/myproject-backup.avfs

# Or copy it to the vaults directory
cp ~/backups/myproject-backup.avfs ~/.avfs/vaults/restored.avfs
avfs vault use restored
```

## Importing External Databases

Register an existing avfs database:

```bash
avfs vault import <name> <path>
```

```bash
$ avfs vault import colleague-project ~/Downloads/shared-project.avfs
Registered vault 'colleague-project'
```

This creates a reference to the external database without copying it.

## Vault Portability

### Moving Vaults Between Machines

1. Copy the `.avfs` file to the target machine
2. Register it with `avfs vault import`

```bash
# On source machine
scp ~/.avfs/vaults/myproject.avfs user@target:~/myproject.avfs

# On target machine
avfs vault import myproject ~/myproject.avfs
```

### Cloud Sync

Vault files can be synced via cloud storage:

```bash
# Store vault in Dropbox
avfs vault create shared --path ~/Dropbox/avfs/shared.avfs

# Note: Avoid simultaneous access from multiple machines
```

**Warning:** SQLite databases shouldn't be accessed concurrently from multiple machines. Use proper locking or sync only when not in use.

## Troubleshooting

### Vault Locked

```
Error: Vault is locked by another process
```

Another avfs instance is using the vault. Check for:
- Other terminal sessions
- Background avfs processes
- Stale lock files (in `~/.avfs/locks/`)

### Corrupt Vault

If a vault becomes corrupted:

```bash
# Check integrity
sqlite3 ~/.avfs/vaults/damaged.avfs "PRAGMA integrity_check"

# Attempt recovery
sqlite3 ~/.avfs/vaults/damaged.avfs ".recover" | sqlite3 recovered.avfs
avfs vault import recovered recovered.avfs
```

### Vault Not Found

```
Error: Vault 'missing' not found
```

Check if the vault exists:

```bash
avfs vault list
ls ~/.avfs/vaults/
```

If the database file exists but isn't registered, import it:

```bash
avfs vault import missing ~/.avfs/vaults/missing.avfs
```
