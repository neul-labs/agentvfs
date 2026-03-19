# Vault Management

A **vault** is an independent virtual filesystem stored in a single database file (`.vfs` extension). vfs supports multiple vaults, allowing you to organize different projects, experiments, or backups in isolated containers.

## Concepts

### What is a Vault?

- A complete virtual filesystem in one `.vfs` file
- Contains files, directories, versions, tags, and metadata
- Fully isolated from other vaults
- Portable - copy the file to move your filesystem

### Default Storage Location

```
~/.vfs/
├── config.toml          # Global vfs configuration
├── current              # Name of the active vault
└── vaults/
    ├── default.vfs       # The default vault
    ├── myproject.vfs     # User-created vault
    └── experiments.vfs   # Another vault
```

## Creating Vaults

### Basic Creation

```bash
vfs vault create <name>
```

Creates a new vault in the default location (`~/.vfs/vaults/<name>.vfs`).

```bash
$ vfs vault create myproject
Created vault 'myproject' at ~/.vfs/vaults/myproject.vfs
Switched to vault 'myproject'
```

### Custom Location

```bash
vfs vault create <name> --path <path>
```

Store the vault database at a custom location:

```bash
# External drive
vfs vault create backup --path /mnt/external/backup.vfs

# Project directory
vfs vault create project-files --path ./project.vfs

# Absolute path
vfs vault create archive --path /data/archives/2024.vfs
```

### Create Without Switching

```bash
vfs vault create experiments --no-switch
```

Creates the vault but keeps the current vault active.

## Listing Vaults

```bash
$ vfs vault list
  VAULT          SIZE      FILES    VERSIONS   PATH
* default        12.3 MB   234      1,892      ~/.vfs/vaults/default.vfs
  myproject      4.5 MB    89       456        ~/.vfs/vaults/myproject.vfs
  experiments    128 KB    12       24         ~/.vfs/vaults/experiments.vfs
  backup         2.1 GB    5,678    45,678     /mnt/external/backup.vfs

* = active vault
```

### JSON Output

```bash
vfs vault list --json
```

Useful for scripting:

```json
[
  {
    "name": "default",
    "path": "~/.vfs/vaults/default.vfs",
    "size_bytes": 12902400,
    "file_count": 234,
    "version_count": 1892,
    "active": true
  }
]
```

## Switching Vaults

```bash
vfs vault use <name>
```

Switch to a different vault:

```bash
$ vfs vault use myproject
Switched to vault 'myproject'
Current directory: /

$ vfs pwd
/
```

### Temporary Vault Override

Use `--vault` flag with any command to temporarily use a different vault:

```bash
# List files in another vault without switching
vfs --vault backup ls /documents

# Copy between vaults
vfs cat /file.txt | vfs --vault backup write /file.txt
```

## Vault Information

```bash
vfs vault info [name]
```

Show detailed information about a vault:

```bash
$ vfs vault info myproject
Vault: myproject
Path: ~/.vfs/vaults/myproject.vfs
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
vfs vault delete <name>
```

Delete a vault permanently:

```bash
$ vfs vault delete experiments
This will permanently delete vault 'experiments' and all its contents.
Are you sure? [y/N] y
Deleted vault 'experiments'
```

### Force Delete

Skip confirmation prompt:

```bash
vfs vault delete experiments --force
```

### Cannot Delete Active Vault

```bash
$ vfs vault delete myproject
Error: Cannot delete the active vault.
Switch to another vault first: vfs vault use default
```

## Vault Configuration

Each vault has its own settings stored in the `settings` table.

### View Settings

```bash
vfs vault config
```

### Modify Settings

```bash
vfs vault config <key> <value>
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
vfs vault config prune_keep_count 20

# Switch to time-based pruning
vfs vault config prune_strategy time_based
vfs vault config prune_max_age_days 90
```

## Backup and Restore

### Backup a Vault

Since a vault is a single SQLite file, backup is simple:

```bash
# Copy the database file
cp ~/.vfs/vaults/myproject.vfs ~/backups/myproject-$(date +%Y%m%d).vfs

# Or use vfs export for a tarball
vfs export / ~/backups/myproject-export.tar.gz --recursive
```

### Online Backup

For backing up while vfs is in use:

```bash
sqlite3 ~/.vfs/vaults/myproject.vfs ".backup ~/backups/myproject.vfs"
```

### Restore from Backup

```bash
# Register the backup as a vault
vfs vault create restored --path ~/backups/myproject-backup.vfs

# Or copy it to the vaults directory
cp ~/backups/myproject-backup.vfs ~/.vfs/vaults/restored.vfs
vfs vault use restored
```

## Importing External Databases

Register an existing vfs database:

```bash
vfs vault import <name> <path>
```

```bash
$ vfs vault import colleague-project ~/Downloads/shared-project.vfs
Registered vault 'colleague-project'
```

This creates a reference to the external database without copying it.

## Vault Portability

### Moving Vaults Between Machines

1. Copy the `.vfs` file to the target machine
2. Register it with `vfs vault import`

```bash
# On source machine
scp ~/.vfs/vaults/myproject.vfs user@target:~/myproject.vfs

# On target machine
vfs vault import myproject ~/myproject.vfs
```

### Cloud Sync

Vault files can be synced via cloud storage:

```bash
# Store vault in Dropbox
vfs vault create shared --path ~/Dropbox/vfs/shared.vfs

# Note: Avoid simultaneous access from multiple machines
```

**Warning:** SQLite databases shouldn't be accessed concurrently from multiple machines. Use proper locking or sync only when not in use.

## Troubleshooting

### Vault Locked

```
Error: Vault is locked by another process
```

Another vfs instance is using the vault. Check for:
- Other terminal sessions
- Background vfs processes
- Stale lock files (in `~/.vfs/locks/`)

### Corrupt Vault

If a vault becomes corrupted:

```bash
# Check integrity
sqlite3 ~/.vfs/vaults/damaged.vfs "PRAGMA integrity_check"

# Attempt recovery
sqlite3 ~/.vfs/vaults/damaged.vfs ".recover" | sqlite3 recovered.vfs
vfs vault import recovered recovered.vfs
```

### Vault Not Found

```
Error: Vault 'missing' not found
```

Check if the vault exists:

```bash
vfs vault list
ls ~/.vfs/vaults/
```

If the database file exists but isn't registered, import it:

```bash
vfs vault import missing ~/.vfs/vaults/missing.vfs
```
