# FUSE Mount

VFS can mount a vault as a real directory using FUSE (Filesystem in Userspace), enabling use of standard tools like `ls`, `cat`, `vim`, and others on virtual files.

!!! note "Feature Flag Required"
    FUSE support requires building with the `fuse` feature and installing libfuse3.

## Requirements

### Linux

```bash
# Ubuntu/Debian
sudo apt-get install libfuse3-dev

# Fedora
sudo dnf install fuse3-devel

# Arch
sudo pacman -S fuse3
```

### macOS

```bash
brew install macfuse
```

### Build with FUSE Support

```bash
cargo build --release --features fuse
```

## Mounting a Vault

### Basic Mount

```bash
# Create mount point
mkdir -p /tmp/vfs-mount

# Mount vault
vfs mount <vault-name> <mountpoint> --foreground
```

**Example:**

```bash
$ vfs mount myproject /tmp/vfs-mount --foreground
Mounting myproject at /tmp/vfs-mount (foreground mode)
Press Ctrl+C to unmount
```

### Mount Options

| Option | Description |
|--------|-------------|
| `--foreground` | Run in foreground (for debugging) |
| `--readonly` | Mount read-only |
| `--allow-other` | Allow other users to access |

**Examples:**

```bash
# Read-only mount
vfs mount myproject /tmp/vfs-mount --foreground --readonly

# Allow other users (requires /etc/fuse.conf configuration)
vfs mount myproject /tmp/vfs-mount --foreground --allow-other
```

## Using the Mount

Once mounted, use standard Unix tools:

### List Files

```bash
ls -la /tmp/vfs-mount/
```

### Read Files

```bash
cat /tmp/vfs-mount/docs/readme.txt
less /tmp/vfs-mount/docs/readme.txt
```

### Write Files

```bash
echo "New content" > /tmp/vfs-mount/newfile.txt
```

### Edit with Editors

```bash
vim /tmp/vfs-mount/docs/readme.txt
nano /tmp/vfs-mount/config.yaml
code /tmp/vfs-mount/  # VS Code
```

### Create Directories

```bash
mkdir /tmp/vfs-mount/newdir
mkdir -p /tmp/vfs-mount/path/to/nested
```

### Copy and Move

```bash
cp /tmp/vfs-mount/file.txt /tmp/vfs-mount/backup.txt
mv /tmp/vfs-mount/old.txt /tmp/vfs-mount/new.txt
```

### Delete

```bash
rm /tmp/vfs-mount/unwanted.txt
rm -r /tmp/vfs-mount/old-directory/
```

## Unmounting

### Using vfs unmount

```bash
vfs unmount /tmp/vfs-mount
```

### Using fusermount

```bash
fusermount -u /tmp/vfs-mount

# Lazy unmount (if busy)
fusermount -uz /tmp/vfs-mount
```

### Ctrl+C (Foreground Mode)

If running in foreground mode, press `Ctrl+C` to unmount.

## Synchronization

Changes made through the FUSE mount are immediately visible via VFS commands:

```bash
# Create file via FUSE
echo "Hello" > /tmp/vfs-mount/hello.txt

# Verify via VFS command
vfs --vault myproject cat /hello.txt
# Output: Hello
```

And vice versa:

```bash
# Create file via VFS
vfs --vault myproject write /world.txt "World"

# Read via FUSE
cat /tmp/vfs-mount/world.txt
# Output: World
```

## Version History

Writes through FUSE create new versions just like VFS commands:

```bash
# Write multiple times
echo "Version 1" > /tmp/vfs-mount/file.txt
echo "Version 2" > /tmp/vfs-mount/file.txt
echo "Version 3" > /tmp/vfs-mount/file.txt

# Check version history
vfs --vault myproject log /file.txt
```

## Limitations

1. **No symlinks**: VFS doesn't support symbolic links
2. **No hard links**: Not supported
3. **No extended attributes**: Use `vfs meta` instead
4. **Single user**: No multi-user permission model
5. **No concurrent mount**: Don't mount the same vault twice

## Use Cases

### IDE Integration

Mount a vault and open in your IDE:

```bash
vfs mount myproject ~/vfs-project --foreground &
code ~/vfs-project
```

### Backup Tools

Use standard backup tools on virtual files:

```bash
vfs mount myproject /tmp/mount --foreground --readonly &
rsync -av /tmp/mount/ /backup/destination/
```

### Shell Scripts

Process virtual files with shell scripts:

```bash
vfs mount myproject /tmp/mount --foreground &
for f in /tmp/mount/data/*.csv; do
    process_csv "$f"
done
vfs unmount /tmp/mount
```

## Troubleshooting

### Permission Denied

```bash
fusermount: user has no write access to mountpoint
```

Ensure you have write permission to the mount directory:

```bash
mkdir -p /tmp/vfs-mount
chmod 755 /tmp/vfs-mount
```

### Transport Endpoint Not Connected

```bash
ls: cannot access '/tmp/vfs-mount': Transport endpoint is not connected
```

The mount crashed or was improperly terminated. Force unmount:

```bash
fusermount -uz /tmp/vfs-mount
```

### FUSE Not Available

```bash
fuse: device not found
```

Load the FUSE kernel module:

```bash
sudo modprobe fuse
```

### allow_other Error

```bash
fusermount3: option allow_other only allowed if 'user_allow_other' is set in /etc/fuse.conf
```

Edit `/etc/fuse.conf` and uncomment `user_allow_other`, or don't use `--allow-other`.
