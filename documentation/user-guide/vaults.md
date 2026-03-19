# Vault Management

A **vault** is an independent virtual filesystem stored in a single database file (`.avfs` extension). VFS supports multiple vaults, allowing you to organize different projects, experiments, or backups in isolated containers.

## What is a Vault?

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

Store the vault database at a custom location:

```bash
# External drive
avfs vault create backup --path /mnt/external/backup.avfs

# Project directory
avfs vault create project-files --path ./project.avfs
```

## Listing Vaults

```bash
$ avfs vault list
  VAULT          SIZE      FILES    VERSIONS   PATH
* default        12.3 MB   234      1,892      ~/.avfs/vaults/default.avfs
  myproject      4.5 MB    89       456        ~/.avfs/vaults/myproject.avfs
  experiments    128 KB    12       24         ~/.avfs/vaults/experiments.avfs

* = active vault
```

### JSON Output

```bash
avfs vault list --json
```

## Switching Vaults

```bash
avfs vault use <name>
```

Switch to a different vault:

```bash
$ avfs vault use myproject
Switched to vault 'myproject'
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

!!! warning "Cannot Delete Active Vault"
    You cannot delete the currently active vault. Switch to another vault first.

## Backup and Restore

### Backup a Vault

Since a vault is a single SQLite file, backup is simple:

```bash
# Copy the database file
cp ~/.avfs/vaults/myproject.avfs ~/backups/myproject-$(date +%Y%m%d).avfs
```

### Online Backup

For backing up while VFS is in use:

```bash
sqlite3 ~/.avfs/vaults/myproject.avfs ".backup ~/backups/myproject.avfs"
```

### Restore from Backup

```bash
# Copy backup to vaults directory
cp ~/backups/myproject-backup.avfs ~/.avfs/vaults/restored.avfs
avfs vault use restored
```

## Vault Portability

### Moving Vaults Between Machines

1. Copy the `.avfs` file to the target machine
2. Place it in `~/.avfs/vaults/` or use a custom path

```bash
# On source machine
scp ~/.avfs/vaults/myproject.avfs user@target:~/.avfs/vaults/

# On target machine
avfs vault use myproject
```

### Cloud Sync

Vault files can be synced via cloud storage:

```bash
# Store vault in Dropbox
avfs vault create shared --path ~/Dropbox/avfs/shared.avfs
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
sqlite3 ~/.avfs/vaults/damaged.avfs "PRAGMA integrity_check"

# Attempt recovery
sqlite3 ~/.avfs/vaults/damaged.avfs ".recover" | sqlite3 recovered.avfs
```
