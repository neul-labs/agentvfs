# Python SDK

The `agentvfs-cli` PyPI package ships a Python client for embedding agentvfs into Python agent runtimes. The client wraps the `avfs` CLI's JSON output, giving you a typed, Pythonic interface backed by a real binary — no separate IPC server.

## Install

```bash
pip install agentvfs-cli
```

The package distribution name is `agentvfs-cli`, but the importable module is `agentvfs`:

```python
from agentvfs import AVFS, AVFSError, VaultNotFoundError
```

The native binary is resolved lazily — see [the installation guide](../getting-started/installation.md#pip) for the full resolution order, cache location, and `AGENTVFS_BIN` override.

## Quick start

```python
from agentvfs import AVFS

vfs = AVFS("myproject")
vfs.create_vault("myproject")
vfs.mkdir("/src")
vfs.write("/src/main.py", "print('hello')")

print(vfs.cat("/src/main.py"))
print(vfs.ls("/src"))
```

The `AVFS` constructor takes a vault name; that vault is implicitly passed as `--vault <name>` to every CLI invocation.

```python
AVFS(vault: str | None = None, *, json_output: bool = True, binary: str | None = None)
```

| Argument      | Default | Meaning                                                                |
|---------------|---------|------------------------------------------------------------------------|
| `vault`       | `None`  | Vault name (passed as `--vault`). `None` uses the global current vault. |
| `json_output` | `True`  | Pass `--json` to all commands and parse stdout as JSON.                 |
| `binary`      | auto    | Override the resolved `avfs` binary path. Same precedence as `AGENTVFS_BIN`. |

## Errors

All errors raise `AVFSError` (or a subclass). The error type comes from the CLI's structured JSON output:

```python
from agentvfs import AVFS, AVFSError, VaultNotFoundError

vfs = AVFS("nonexistent")
try:
    vfs.ls("/")
except VaultNotFoundError as e:
    print(f"Vault missing: {e}")
except AVFSError as e:
    print(f"Other failure: {e} (type={e.error_type})")
```

## API reference

### Vault management

| Method                                       | CLI equivalent                |
|----------------------------------------------|-------------------------------|
| `create_vault(name)`                         | `avfs vault create <name>`    |
| `use_vault(name)`                            | `avfs vault use <name>`       |
| `fork_vault(source, name)`                   | `avfs vault fork <src> <new>` |
| `list_vaults() -> list[str]`                 | `avfs vault list`             |

### Filesystem

| Method                                       | CLI equivalent                |
|----------------------------------------------|-------------------------------|
| `mkdir(path)`                                | `avfs mkdir <path>`           |
| `write(path, content: str \| bytes)`         | `avfs write <path> --stdin`   |
| `cat(path) -> str`                           | `avfs cat <path>`             |
| `ls(path="/") -> list[dict]`                 | `avfs ls <path>`              |
| `rm(path, *, recursive=False)`               | `avfs rm [-r] <path>`         |
| `cp(src, dst)`                               | `avfs cp <src> <dst>`         |
| `mv(src, dst)`                               | `avfs mv <src> <dst>`         |
| `tree(path="/") -> str`                      | `avfs tree <path>`            |

### Search

| Method                                                       | CLI equivalent                  |
|--------------------------------------------------------------|---------------------------------|
| `grep(pattern, path="/") -> list[dict]`                      | `avfs grep <pattern> <path>`    |
| `search(query, path=None) -> list[dict]`                     | `avfs search <query> [<path>]`  |

### Checkpoints

| Method                                       | CLI equivalent                  |
|----------------------------------------------|---------------------------------|
| `checkpoint_save(name)`                      | `avfs checkpoint save <name>`   |
| `checkpoint_restore(name)`                   | `avfs checkpoint restore <name>`|
| `checkpoint_list() -> list[dict]`            | `avfs checkpoint list`          |

### Proxy execution

```python
proxy_exec(*command, cwd="/", readonly=False, timeout=None, checkpoint=True) -> dict
```

Wraps `avfs proxy exec`. Returns the structured `ExecutionEnvelope` (see [Architecture](../reference/architecture.md#executionenvelope) for the schema):

```python
result = vfs.proxy_exec("python", "/src/main.py", cwd="/src", checkpoint=True)
print(result["execution_result"]["stdout"])
print(result["execution_result"]["exit_code"])
print(result["execution_result"]["changed_files"])
```

| Argument     | Meaning                                                              |
|--------------|----------------------------------------------------------------------|
| `*command`   | The argv-style command and its arguments                             |
| `cwd`        | Working directory inside the mounted vault (default `/`)             |
| `readonly`   | Mount the workspace read-only                                        |
| `timeout`    | Timeout in milliseconds (default 300_000)                            |
| `checkpoint` | (Reserved — `--no-checkpoint` is not yet wired in the CLI)           |

### Maintenance

| Method                  | CLI equivalent      |
|-------------------------|---------------------|
| `stats()`               | `avfs stats`        |
| `gc()`                  | `avfs gc`           |
| `compact()`             | `avfs compact`      |

## Patterns

### Forked task workspaces

The recommended pattern for agent tasks: fork a durable root vault, work in the fork, save checkpoints between risky operations, and discard or merge the fork at the end.

```python
from agentvfs import AVFS

root = AVFS()                          # global current vault
root.create_vault("myproject")
root.fork_vault("myproject", "myproject-task-1")

task = AVFS("myproject-task-1")
task.checkpoint_save("baseline")

task.write("/src/feature.py", "# new feature")
result = task.proxy_exec("pytest", "/tests", cwd="/")

if result["execution_result"]["exit_code"] != 0:
    task.checkpoint_restore("baseline")
```

### Streaming proxy results

`proxy_exec` is synchronous — for long-running commands, prefer the CLI directly with `subprocess.Popen` and parse the trailing JSON envelope. The Python SDK is best for short, well-bounded operations where a structured return value is what you want.

### Custom binary path

For testing against a locally-built dev binary:

```python
from agentvfs import AVFS

vfs = AVFS("dev-vault", binary="/path/to/target/release/avfs")
```

This overrides the resolution order and bypasses both PATH and the cache.

## Versioning

The SDK's version is the same as the PyPI package version:

```python
import agentvfs
print(agentvfs.__version__)
```

Read at runtime from package metadata (`importlib.metadata`), so it never drifts from the installed dist.

## Related guides

- [Installation](../getting-started/installation.md) — install channels and binary resolution
- [Agent Integration](agent-integration.md) — proxy boundary and integration shapes
- [Proxy Boundary](proxy-boundary.md) — the conceptual model behind `proxy_exec`
- [Architecture](../reference/architecture.md) — execution envelope schema
