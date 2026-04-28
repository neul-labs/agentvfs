# agentvfs

Python wrapper for the AgentVFS (`avfs`) CLI.

## Install

```bash
pip install agentvfs
```

Requires the `avfs` binary to be installed (downloaded automatically if using the npm wrapper, or install via `cargo install agentvfs`).

## Usage

```python
from agentvfs import AVFS

avfs = AVFS("myproject")
avfs.mkdir("/src")
avfs.write("/src/main.py", "print('hello')")
result = avfs.proxy_exec("python", "/src/main.py")
print(result["stdout"])
```

## API

- Vaults: `create_vault`, `use_vault`, `fork_vault`, `list_vaults`
- Filesystem: `mkdir`, `write`, `cat`, `ls`, `rm`, `cp`, `mv`, `tree`
- Proxy: `proxy_exec`
- Checkpoints: `checkpoint_save`, `checkpoint_restore`, `checkpoint_list`
- Search: `grep`, `search`
- Maintenance: `stats`, `gc`, `compact`
