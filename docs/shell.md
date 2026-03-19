# Interactive Shell

avfs provides an interactive shell mode where you can use commands without the `avfs` prefix, offering a more natural filesystem experience.

## Starting the Shell

```bash
avfs shell
```

This launches an interactive REPL:

```bash
$ avfs shell
avfs 1.0.0 - SQLite Virtual Filesystem
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

All avfs commands work in the shell:

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

Command history is saved to `~/.avfs/history` and persists across sessions.

Configure history size:

```bash
avfs config history_size 1000
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

Save aliases to `~/.avfs/aliases`:

```bash
# ~/.avfs/aliases
alias ll='ls -l'
alias la='ls -la'
alias ..='cd ..'
alias ...='cd ../..'
```

## Shell Scripting

The shell supports basic scripting:

### Run Script

```bash
avfs shell < script.avfs
```

### Script Example

```bash
# setup.avfs
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
$ avfs aliases
alias vls='avfs ls'
alias vcd='avfs cd'
alias vcat='avfs cat'
...

# Add to shell rc
$ avfs aliases >> ~/.bashrc
$ source ~/.bashrc

# Now use directly
$ vls /docs
$ vcat /docs/readme.txt
```

### Custom Prefix

```bash
$ avfs aliases --prefix "v"
alias vls='avfs ls'
alias vcat='avfs cat'
...

$ avfs aliases --prefix "avfs-"
alias avfs-ls='avfs ls'
alias avfs-cat='avfs cat'
```

### Shell-Specific Formats

```bash
avfs aliases --format bash   # Bash aliases (default)
avfs aliases --format zsh    # Zsh aliases
avfs aliases --format fish   # Fish abbreviations
```

## Configuration

### Shell Settings

Configure shell behavior in `~/.avfs/config.toml`:

```toml
[shell]
# Prompt format
prompt = "[{vault}] {path} > "

# Enable colors
color = true

# History settings
history_size = 1000
history_file = "~/.avfs/history"

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
avfs shell --verbose
```

### Completion Not Working

Ensure completion is enabled:

```bash
avfs config completion true
```

### History Not Saving

Check history file permissions:

```bash
ls -la ~/.avfs/history
```

### Slow Startup

Disable features for faster startup:

```bash
avfs shell --no-history --no-completion
```

Or in config:

```toml
[shell]
history_size = 0
completion = false
```
