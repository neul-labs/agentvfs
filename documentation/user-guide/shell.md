# Interactive Shell

VFS provides an interactive shell mode where you can use commands without the `avfs` prefix.

## Starting the Shell

```bash
avfs shell
```

This launches an interactive REPL:

```bash
$ avfs shell
avfs - Virtual Filesystem Shell
Type 'help' for commands, 'exit' to quit.

[default] / >
```

## Prompt Format

The prompt shows:

- **Vault name**: Current active vault
- **Current path**: Working directory within the vault

```
[vault-name] /current/path >
```

**Examples:**

```
[default] / >
[myproject] /src >
[backup] /documents/2024 >
```

## Using Commands

In shell mode, commands work without the `avfs` prefix:

```bash
[default] / > ls
docs/
src/
config/

[default] / > cd docs

[default] /docs > cat readme.txt
Welcome to the project!

[default] /docs > mkdir archive

[default] /docs > cp readme.txt archive/

[default] /docs > ls archive/
readme.txt
```

## Available Commands

All VFS commands work in the shell:

### Navigation

| Command | Description |
|---------|-------------|
| `ls [path]` | List directory contents |
| `cd [path]` | Change directory |
| `pwd` | Print working directory |
| `tree [path]` | Display directory tree |

### File Operations

| Command | Description |
|---------|-------------|
| `cat <file>` | Display file contents |
| `write <file> text` | Write text to file |
| `cp <src> <dst>` | Copy files |
| `mv <src> <dst>` | Move/rename files |
| `rm <path>` | Remove files |
| `mkdir <dir>` | Create directory |

### Search

| Command | Description |
|---------|-------------|
| `grep <pat> [path]` | Search file contents |
| `find [path] [opt]` | Find files |
| `search <query>` | Full-text search |

### Versioning

| Command | Description |
|---------|-------------|
| `log <file>` | Show version history |
| `checkout <f> -v <v>` | Restore version |
| `revert <file>` | Revert to previous |
| `diff <f1> <f2>` | Compare files |

### Vault

| Command | Description |
|---------|-------------|
| `vault list` | List vaults |
| `vault use <name>` | Switch vault |
| `vault info` | Vault information |

### Shell

| Command | Description |
|---------|-------------|
| `help` | Show help |
| `exit` | Exit shell (or Ctrl+D) |
| `clear` | Clear screen |

## Tab Completion

The shell supports tab completion for:

### Commands

```bash
[default] / > gr<TAB>
grep
```

### Paths

```bash
[default] / > cd do<TAB>
[default] / > cd docs/

[default] /docs > cat re<TAB>
[default] /docs > cat readme.txt
```

## Command History

### Navigate History

- **Up Arrow**: Previous command
- **Down Arrow**: Next command
- **Ctrl+R**: Reverse search history

### History Persistence

Command history is saved to `~/.avfs/history` and persists across sessions.

Disable history with:

```bash
avfs shell --no-history
```

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Ctrl+A` | Move to start of line |
| `Ctrl+E` | Move to end of line |
| `Ctrl+U` | Clear line before cursor |
| `Ctrl+K` | Clear line after cursor |
| `Ctrl+W` | Delete word before cursor |
| `Ctrl+L` | Clear screen |
| `Ctrl+C` | Cancel current command |
| `Ctrl+D` | Exit shell (if line empty) |
| `Tab` | Auto-complete |

## Shell Aliases

Generate aliases for your regular shell:

```bash
$ avfs aliases
alias vls='avfs ls'
alias vcat='avfs cat'
alias vwrite='avfs write'
...
```

### Add to Shell Configuration

=== "Bash"

    ```bash
    # Add to ~/.bashrc
    eval "$(avfs aliases --format bash)"
    ```

=== "Zsh"

    ```bash
    # Add to ~/.zshrc
    eval "$(avfs aliases --format zsh)"
    ```

=== "Fish"

    ```bash
    # Add to ~/.config/fish/config.fish
    avfs aliases --format fish | source
    ```

Then use directly from your shell:

```bash
$ vls /docs
$ vcat /docs/readme.txt
```

## Tips

### Quick Directory Navigation

```bash
# Go to root
cd /

# Go up one level
cd ..

# Go to specific path
cd /src/components
```

### Use Relative Paths

In the shell, relative paths work:

```bash
[default] /docs > cat ./readme.txt
[default] /docs > ls ../src/
```

### Switch Vaults Quickly

```bash
[default] / > vault use myproject
[myproject] / > vault use default
[default] / >
```
