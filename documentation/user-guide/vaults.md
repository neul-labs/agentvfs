# Vault Management

A **vault** is an independent virtual filesystem stored in a single database file (`.vfs` extension). VFS supports multiple vaults, allowing you to organize different projects, experiments, or backups in isolated containers.

## What is a Vault?

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

Store the vault database at a custom location:

```bash
# External drive
vfs vault create backup --path /mnt/external/backup.vfs

# Project directory
vfs vault create project-files --path ./project.vfs
```

## Listing Vaults

```bash
$ vfs vault list
  VAULT          SIZE      FILES    VERSIONS   PATH
* default        12.3 MB   234      1,892      ~/.vfs/vaults/default.vfs
  myproject      4.5 MB    89       456        ~/.vfs/vaults/myproject.vfs
  experiments    128 KB    12       24         ~/.vfs/vaults/experiments.vfs

* = active vault
```

### JSON Output

```bash
vfs vault list --json
```

## Switching Vaults

```bash
vfs vault use <name>
```

Switch to a different vault:

```bash
$ vfs vault use myproject
Switched to vault 'myproject'
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

!!! warning "Cannot Delete Active Vault"
    You cannot delete the currently active vault. Switch to another vault first.

## Backup and Restore

### Backup a Vault

Since a vault is a single SQLite file, backup is simple:

```bash
# Copy the database file
cp ~/.vfs/vaults/myproject.vfs ~/backups/myproject-$(date +%Y%m%d).vfs
```

### Online Backup

For backing up while VFS is in use:

```bash
sqlite3 ~/.vfs/vaults/myproject.vfs ".backup ~/backups/myproject.vfs"
```

### Restore from Backup

```bash
# Copy backup to vaults directory
cp ~/backups/myproject-backup.vfs ~/.vfs/vaults/restored.vfs
vfs vault use restored
```

## Vault Portability

### Moving Vaults Between Machines

1. Copy the `.vfs` file to the target machine
2. Place it in `~/.vfs/vaults/` or use a custom path

```bash
# On source machine
scp ~/.vfs/vaults/myproject.vfs user@target:~/.vfs/vaults/

# On target machine
vfs vault use myproject
```

### Cloud Sync

Vault files can be synced via cloud storage:

```bash
# Store vault in Dropbox
vfs vault create shared --path ~/Dropbox/vfs/shared.vfs
```

!!! warning "Concurrent Access"
    SQLite databases shouldn't be accessed concurrently from multiple machines.
    Use proper locking or sync only when not in use.

## Troubleshooting

### Vault Locked

```
Error: Vault is locked by another process
```

Another VFS instance is using the vault. Check for:

- Other terminal sessions
- Background VFS processes
- Stale lock files

### Corrupt Vault

If a vault becomes corrupted:

```bash
# Check integrity
sqlite3 ~/.vfs/vaults/damaged.vfs "PRAGMA integrity_check"

# Attempt recovery
sqlite3 ~/.vfs/vaults/damaged.vfs ".recover" | sqlite3 recovered.vfs
```
