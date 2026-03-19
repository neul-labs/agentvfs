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
mkdir -p /tmp/avfs-mount

# Mount vault
avfs mount <vault-name> <mountpoint> --foreground
```

**Example:**

```bash
$ avfs mount myproject /tmp/avfs-mount --foreground
Mounting myproject at /tmp/avfs-mount (foreground mode)
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
avfs mount myproject /tmp/avfs-mount --foreground --readonly

# Allow other users (requires /etc/fuse.conf configuration)
avfs mount myproject /tmp/avfs-mount --foreground --allow-other
```

## Using the Mount

Once mounted, use standard Unix tools:

### List Files

```bash
ls -la /tmp/avfs-mount/
```

### Read Files

```bash
cat /tmp/avfs-mount/docs/readme.txt
less /tmp/avfs-mount/docs/readme.txt
```

### Write Files

```bash
echo "New content" > /tmp/avfs-mount/newfile.txt
```

### Edit with Editors

```bash
vim /tmp/avfs-mount/docs/readme.txt
nano /tmp/avfs-mount/config.yaml
code /tmp/avfs-mount/  # VS Code
```

### Create Directories

```bash
mkdir /tmp/avfs-mount/newdir
mkdir -p /tmp/avfs-mount/path/to/nested
```

### Copy and Move

```bash
cp /tmp/avfs-mount/file.txt /tmp/avfs-mount/backup.txt
mv /tmp/avfs-mount/old.txt /tmp/avfs-mount/new.txt
```

### Delete

```bash
rm /tmp/avfs-mount/unwanted.txt
rm -r /tmp/avfs-mount/old-directory/
```

## Unmounting

### Using avfs unmount

```bash
avfs unmount /tmp/avfs-mount
```

### Using fusermount

```bash
fusermount -u /tmp/avfs-mount

# Lazy unmount (if busy)
fusermount -uz /tmp/avfs-mount
```

### Ctrl+C (Foreground Mode)

If running in foreground mode, press `Ctrl+C` to unmount.

## Synchronization

Changes made through the FUSE mount are immediately visible via VFS commands:

```bash
# Create file via FUSE
echo "Hello" > /tmp/avfs-mount/hello.txt

# Verify via VFS command
avfs --vault myproject cat /hello.txt
# Output: Hello
```

And vice versa:

```bash
# Create file via VFS
avfs --vault myproject write /world.txt "World"

# Read via FUSE
cat /tmp/avfs-mount/world.txt
# Output: World
```

## Version History

Writes through FUSE create new versions just like VFS commands:

```bash
# Write multiple times
echo "Version 1" > /tmp/avfs-mount/file.txt
echo "Version 2" > /tmp/avfs-mount/file.txt
echo "Version 3" > /tmp/avfs-mount/file.txt

# Check version history
avfs --vault myproject log /file.txt
```

## Limitations

1. **No symlinks**: VFS doesn't support symbolic links
2. **No hard links**: Not supported
3. **No extended attributes**: Use `avfs meta` instead
4. **Single user**: No multi-user permission model
5. **No concurrent mount**: Don't mount the same vault twice

## Use Cases

### IDE Integration

Mount a vault and open in your IDE:

```bash
avfs mount myproject ~/avfs-project --foreground &
code ~/avfs-project
```

### Backup Tools

Use standard backup tools on virtual files:

```bash
avfs mount myproject /tmp/mount --foreground --readonly &
rsync -av /tmp/mount/ /backup/destination/
```

### Shell Scripts

Process virtual files with shell scripts:

```bash
avfs mount myproject /tmp/mount --foreground &
for f in /tmp/mount/data/*.csv; do
    process_csv "$f"
done
avfs unmount /tmp/mount
```

## Troubleshooting

### Permission Denied

```bash
fusermount: user has no write access to mountpoint
```

Ensure you have write permission to the mount directory:

```bash
mkdir -p /tmp/avfs-mount
chmod 755 /tmp/avfs-mount
```

### Transport Endpoint Not Connected

```bash
ls: cannot access '/tmp/avfs-mount': Transport endpoint is not connected
```

The mount crashed or was improperly terminated. Force unmount:

```bash
fusermount -uz /tmp/avfs-mount
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
