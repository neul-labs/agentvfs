# Interactive Shell

agentvfs provides an interactive shell mode for quick exploration and file operations without the `avfs` prefix.

For agent execution, the long-term preferred model is a proxy boundary that mediates top-level shell commands. The interactive shell remains useful for manual inspection and development.

## Starting the Shell

```bash
avfs shell
```

This launches an interactive REPL:

```
$ avfs shell
avfs interactive shell
Type 'help' for available commands, 'exit' to quit.

myproject>
```

## Prompt Format

The prompt shows the current vault name:

```
vault-name>
```

**Examples:**

```
default> ls /
myproject> cat /src/main.py
agent-workspace> tree /
```

## Quick Start

```bash
# Navigate and explore
myproject> ls /
src/
docs/
config.json

myproject> tree /src
/src
├── main.py
├── utils.py
└── tests/
    └── test_main.py

# Read and write files
myproject> cat /config.json
{"debug": true}

myproject> write /notes.txt "Remember to update docs"

# Search across files
myproject> grep "TODO" /src
/src/main.py:42: # TODO: Add error handling
/src/utils.py:15: # TODO: Optimize this

myproject> search "authentication"
/docs/api.md: User authentication is handled by...
```

## Available Commands

### File Operations

| Command | Description |
|---------|-------------|
| `ls [path]` | List directory contents |
| `cat <file>` | Display file contents |
| `write <file> <text>` | Write text to file |
| `cp <src> <dst>` | Copy files or directories |
| `mv <src> <dst>` | Move/rename files |
| `rm <path>` | Remove files or directories |
| `mkdir <dir>` | Create directory |
| `tree [path]` | Display directory tree |
| `pwd` | Print working directory (always `/`) |

### Search

| Command | Description |
|---------|-------------|
| `grep <pattern> [path]` | Search file contents with regex |
| `find [path] [options]` | Find files by name or attributes |
| `search <query>` | Full-text search (FTS5) |

### Versioning

| Command | Description |
|---------|-------------|
| `log <file>` | Show version history |
| `checkout <file> --version <n>` | Restore specific version |
| `revert <file>` | Revert to previous version |
| `diff <file1> <file2>` | Compare files or versions |

### Snapshots

| Command | Description |
|---------|-------------|
| `checkpoint save <name>` | Save current state |
| `checkpoint list` | List checkpoints |
| `checkpoint restore <name>` | Restore to checkpoint |
| `checkpoint delete <name>` | Delete checkpoint |
| `snapshot save <name>` | Save current state |
| `snapshot list` | List all snapshots |
| `snapshot restore <name>` | Restore to snapshot |
| `snapshot delete <name>` | Delete snapshot |

### Vault Management

| Command | Description |
|---------|-------------|
| `vault list` | List all vaults |
| `vault use <name>` | Switch to vault |
| `vault create <name>` | Create new vault |
| `vault delete <name>` | Delete vault |
| `vault info [name]` | Show vault information |

### Import/Export

| Command | Description |
|---------|-------------|
| `import <real-path> <vfs-path>` | Import from real filesystem |
| `export <vfs-path> <real-path>` | Export to real filesystem |
| `exec <command> <vfs-path>` | Run external command on file |

### Maintenance

| Command | Description |
|---------|-------------|
| `stats` | Show storage statistics |
| `prune` | Remove old file versions |
| `gc` | Garbage collect orphaned content |
| `compact` | Compact database |
| `maintain` | Run full maintenance |

### Shell Built-ins

| Command | Description |
|---------|-------------|
| `help` | Show available commands |
| `clear` | Clear screen |
| `exit` / `quit` | Exit shell |

## Tab Completion

The shell supports tab completion for commands and paths:

### Commands

```bash
myproject> gr<TAB>
grep
```

### Paths

```bash
myproject> cat /sr<TAB>
myproject> cat /src/

myproject> cat /src/ma<TAB>
myproject> cat /src/main.py
```

## Command History

### Navigation

- **Up Arrow**: Previous command
- **Down Arrow**: Next command
- **Ctrl+R**: Reverse search history

### Persistence

Command history is saved to `~/.avfs/history` and persists across sessions.

Disable history persistence:

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
| `Ctrl+C` | Cancel current input |
| `Ctrl+D` | Exit shell (if line empty) |
| `Tab` | Auto-complete |

## AI Agent Workflows

The shell is particularly useful for AI agents that need to explore and manipulate files interactively.

For mediated top-level command execution, prefer the proxy-boundary model in the advanced documentation.

### Checkpoint and Rollback

```bash
# Save state before risky operations
agent-workspace> checkpoint save before-refactor

# Make changes
agent-workspace> write /src/main.py "new code..."

# If something goes wrong, rollback
agent-workspace> checkpoint restore before-refactor
```

### Exploring Unknown Codebases

```bash
# Get an overview
agent-workspace> tree /
agent-workspace> stats

# Find relevant files
agent-workspace> search "authentication"
agent-workspace> grep "class.*Handler" /src

# Read and understand
agent-workspace> cat /src/auth.py
agent-workspace> log /src/auth.py
```

### Batch Operations

```bash
# Import a project
agent-workspace> import /path/to/real/project /workspace

# Work on it
agent-workspace> write /workspace/fix.patch "..."

# Export results
agent-workspace> export /workspace /path/to/output
```

## Shell Aliases

Generate aliases for your regular shell to use avfs commands directly:

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

Then use directly:

```bash
$ vls /docs
$ vcat /docs/readme.txt
$ vgrep "TODO" /src
```

## Tips

### JSON Output in Shell

Use `--json` flag for structured output:

```bash
myproject> ls --json /src
{"entries":[{"name":"main.py","type":"file","size":1234},...]}
```

### Quick Vault Switching

```bash
default> vault use myproject
myproject> vault use backup
backup> vault use default
default>
```

### Relative Paths

Relative paths work from the virtual root `/`:

```bash
myproject> cat ./config.json
myproject> ls ../other-vault/   # (doesn't cross vaults)
```
