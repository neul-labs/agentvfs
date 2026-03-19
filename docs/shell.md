# Interactive Shell

vfs provides an interactive shell mode where you can use commands without the `vfs` prefix, offering a more natural filesystem experience.

## Starting the Shell

```bash
vfs shell
```

This launches an interactive REPL:

```bash
$ vfs shell
vfs 1.0.0 - SQLite Virtual Filesystem
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

In shell mode, commands work without the `vfs` prefix:

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

All vfs commands work in the shell:

### Navigation
```
ls [path]          List directory contents
cd [path]          Change directory
pwd                Print working directory
tree [path]        Display directory tree
```

### File Operations
```
cat <file>         Display file contents
write <file> text  Write text to file
cp <src> <dst>     Copy files
mv <src> <dst>     Move/rename files
rm <path>          Remove files
touch <file>       Create empty file
```

### Directories
```
mkdir <dir>        Create directory
rmdir <dir>        Remove empty directory
```

### Search
```
grep <pat> [path]  Search file contents
find [path] [opt]  Find files
search <query>     Full-text search
```

### Versioning
```
log <file>         Show version history
checkout <f> <v>   Restore version
revert <file>      Revert to previous
diff <f1> <f2>     Compare files
```

### Metadata
```
tag <file> <tags>  Add tags
untag <file> <t>   Remove tags
meta <file> [k] v  Get/set metadata
```

### Vault
```
vault list         List vaults
vault use <name>   Switch vault
vault info         Vault information
```

### Maintenance
```
prune [options]    Remove old versions
compact            Reclaim space
gc                 Garbage collection
```

### Shell
```
help               Show help
exit               Exit shell (or Ctrl+D)
clear              Clear screen
history            Show command history
!<n>               Execute history item
```

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

### Options

```bash
[default] / > ls --<TAB>
--all    --long    --recursive    --time    --size
```

### Vault Names

```bash
[default] / > vault use my<TAB>
[default] / > vault use myproject
```

## Command History

### Navigate History

- **Up Arrow**: Previous command
- **Down Arrow**: Next command
- **Ctrl+R**: Reverse search history

### View History

```bash
[default] / > history
  1  ls
  2  cd docs
  3  cat readme.txt
  4  mkdir archive
  5  cp readme.txt archive/
```

### Execute from History

```bash
[default] / > !3
cat readme.txt
Welcome to the project!

[default] / > !!
# Repeats last command
```

### History File

Command history is saved to `~/.vfs/history` and persists across sessions.

Configure history size:

```bash
vfs config history_size 1000
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
| `Ctrl+R` | Reverse search history |
| `Tab` | Auto-complete |

## Aliases

Define custom aliases within the shell:

```bash
[default] / > alias ll='ls -l'
[default] / > alias la='ls -la'
[default] / > ll
TYPE  SIZE     MODIFIED             NAME
d     -        2024-03-10 14:22     docs/
d     -        2024-03-09 10:15     src/
f     1.2 KB   2024-03-08 09:30     README.md
```

### Persistent Aliases

Save aliases to `~/.vfs/aliases`:

```bash
# ~/.vfs/aliases
alias ll='ls -l'
alias la='ls -la'
alias ..='cd ..'
alias ...='cd ../..'
```

## Shell Scripting

The shell supports basic scripting:

### Run Script

```bash
vfs shell < script.vfs
```

### Script Example

```bash
# setup.vfs
mkdir /project
mkdir /project/src
mkdir /project/docs
mkdir /project/tests
touch /project/README.md
write /project/README.md "# My Project"
tag /project/README.md important
```

### Conditional Execution

```bash
# Create if doesn't exist
ls /backup || mkdir /backup

# Chain commands
grep "error" /logs/app.log && tag /logs/app.log has-errors
```

## External Shell Integration

### Generate Aliases

Generate aliases for your regular shell:

```bash
$ vfs aliases
alias vls='vfs ls'
alias vcd='vfs cd'
alias vcat='vfs cat'
...

# Add to shell rc
$ vfs aliases >> ~/.bashrc
$ source ~/.bashrc

# Now use directly
$ vls /docs
$ vcat /docs/readme.txt
```

### Custom Prefix

```bash
$ vfs aliases --prefix "v"
alias vls='vfs ls'
alias vcat='vfs cat'
...

$ vfs aliases --prefix "vfs-"
alias vfs-ls='vfs ls'
alias vfs-cat='vfs cat'
```

### Shell-Specific Formats

```bash
vfs aliases --format bash   # Bash aliases (default)
vfs aliases --format zsh    # Zsh aliases
vfs aliases --format fish   # Fish abbreviations
```

## Configuration

### Shell Settings

Configure shell behavior in `~/.vfs/config.toml`:

```toml
[shell]
# Prompt format
prompt = "[{vault}] {path} > "

# Enable colors
color = true

# History settings
history_size = 1000
history_file = "~/.vfs/history"

# Auto-completion
completion = true

# Editor for multi-line input
editor = "vim"
```

### Custom Prompt

```toml
# Include timestamp
prompt = "[{vault}] {path} ({time}) > "

# Minimal
prompt = "{path}> "

# With username
prompt = "{user}@{vault}:{path}$ "
```

### Color Theme

```toml
[shell.colors]
prompt_vault = "blue"
prompt_path = "green"
error = "red"
warning = "yellow"
directory = "blue"
file = "white"
executable = "green"
```

## Troubleshooting

### Shell Won't Start

Check for configuration errors:

```bash
vfs shell --verbose
```

### Completion Not Working

Ensure completion is enabled:

```bash
vfs config completion true
```

### History Not Saving

Check history file permissions:

```bash
ls -la ~/.vfs/history
```

### Slow Startup

Disable features for faster startup:

```bash
vfs shell --no-history --no-completion
```

Or in config:

```toml
[shell]
history_size = 0
completion = false
```
