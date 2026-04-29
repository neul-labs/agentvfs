"""Console-script entry point for the `agentvfs` command.

Locates the native `avfs` binary on PATH and execs it, forwarding argv,
stdio, and the exit code. The pypi package does not ship the native
binary itself — install it separately via cargo, the install.sh script,
or the npm wrapper. We do not register `avfs` as an entry point on
purpose: doing so would create a Python shim named `avfs` that, if pip's
bin dir is ahead of cargo's on PATH, would resolve to itself via
shutil.which and recurse forever.
"""

from __future__ import annotations

import os
import shutil
import sys


_INSTALL_HINT = (
    "avfs binary not found on PATH.\n"
    "Install one of:\n"
    "  cargo install agentvfs\n"
    "  curl -fsSL https://raw.githubusercontent.com/neul-labs/agentvfs/main/install.sh | bash\n"
    "  npm install -g agentvfs\n"
)


def main() -> "int | None":
    binary = shutil.which("avfs")
    if binary is None:
        sys.stderr.write(_INSTALL_HINT)
        return 127

    # On POSIX, execvp replaces this process so signals/exit codes propagate
    # cleanly. On Windows os.execvp spawns a child and exits the parent,
    # which loses signal forwarding — fall back to subprocess there.
    if os.name == "nt":
        import subprocess

        return subprocess.run([binary, *sys.argv[1:]]).returncode

    os.execvp(binary, [binary, *sys.argv[1:]])
    # execvp does not return on success.
    return 1


if __name__ == "__main__":
    sys.exit(main() or 0)
