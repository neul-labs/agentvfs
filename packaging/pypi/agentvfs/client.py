"""AgentVFS Python client.

Provides a Pythonic interface for interacting with avfs vaults.
"""

import json
import shutil
import subprocess
from pathlib import Path
from typing import Any, Dict, List, Optional, Union


class AVFSError(Exception):
    """Base error for AVFS operations."""

    def __init__(self, message: str, error_type: Optional[str] = None):
        super().__init__(message)
        self.error_type = error_type


class VaultNotFoundError(AVFSError):
    """Raised when a vault does not exist."""

    pass


def _find_avfs() -> str:
    """Locate the avfs binary."""
    avfs = shutil.which("avfs")
    if avfs:
        return avfs
    raise AVFSError(
        "avfs binary not found. Install with: cargo install agentvfs"
    )


class AVFS:
    """Python wrapper for the avfs CLI.

    Example:
        >>> avfs = AVFS("myproject")
        >>> avfs.mkdir("/src")
        >>> avfs.write("/src/main.py", "print('hello')")
        >>> result = avfs.proxy_exec("python", "/src/main.py")
        >>> print(result["stdout"])
    """

    def __init__(
        self,
        vault: Optional[str] = None,
        *,
        json_output: bool = True,
        binary: Optional[str] = None,
    ):
        self.vault = vault
        self.json_output = json_output
        self._binary = binary or _find_avfs()

    def _run(
        self,
        *args: str,
        capture: bool = True,
        input_data: Optional[bytes] = None,
    ) -> Union[Dict[str, Any], str]:
        """Run an avfs command."""
        cmd = [self._binary]
        if self.vault:
            cmd.extend(["--vault", self.vault])
        if self.json_output and capture:
            cmd.append("--json")
        cmd.extend(args)

        result = subprocess.run(
            cmd,
            capture_output=capture,
            text=capture,
            input=input_data.decode() if input_data else None,
        )

        if capture and self.json_output:
            if result.stdout:
                try:
                    data = json.loads(result.stdout)
                except json.JSONDecodeError:
                    data = {"raw": result.stdout}
            else:
                data = {}

            if result.returncode != 0:
                error_type = data.get("error", "Unknown")
                message = data.get("message", result.stderr or "Unknown error")
                if error_type == "VaultNotFound":
                    raise VaultNotFoundError(message, error_type)
                raise AVFSError(message, error_type)

            return data

        if result.returncode != 0:
            raise AVFSError(result.stderr or "Command failed")

        return result.stdout if capture else ""

    # --- Vault management ---

    def create_vault(self, name: str) -> Dict[str, Any]:
        """Create a new vault."""
        return self._run("vault", "create", name)  # type: ignore

    def use_vault(self, name: str) -> Dict[str, Any]:
        """Switch to a vault."""
        return self._run("vault", "use", name)  # type: ignore

    def fork_vault(self, source: str, name: str) -> Dict[str, Any]:
        """Fork a vault."""
        return self._run("vault", "fork", source, name)  # type: ignore

    def list_vaults(self) -> List[str]:
        """List all vaults."""
        result = self._run("vault", "list")
        if isinstance(result, dict):
            return result.get("vaults", [])
        return []

    # --- Filesystem ---

    def mkdir(self, path: str) -> Dict[str, Any]:
        """Create a directory."""
        return self._run("mkdir", path)  # type: ignore

    def write(self, path: str, content: Union[str, bytes]) -> Dict[str, Any]:
        """Write content to a file."""
        if isinstance(content, str):
            content = content.encode()
        return self._run("write", path, "--stdin", input_data=content)  # type: ignore

    def cat(self, path: str) -> str:
        """Read file contents."""
        result = self._run("cat", path)
        if isinstance(result, dict):
            return result.get("content", "")
        return result

    def ls(self, path: str = "/") -> List[Dict[str, Any]]:
        """List directory contents."""
        result = self._run("ls", path)
        if isinstance(result, dict):
            return result.get("entries", [])
        return []

    def rm(self, path: str, *, recursive: bool = False) -> Dict[str, Any]:
        """Remove a file or directory."""
        args = ["rm"]
        if recursive:
            args.append("-r")
        args.append(path)
        return self._run(*args)  # type: ignore

    def cp(self, src: str, dst: str) -> Dict[str, Any]:
        """Copy a file or directory."""
        return self._run("cp", src, dst)  # type: ignore

    def mv(self, src: str, dst: str) -> Dict[str, Any]:
        """Move a file or directory."""
        return self._run("mv", src, dst)  # type: ignore

    def tree(self, path: str = "/") -> str:
        """Display directory tree."""
        result = self._run("tree", path)
        if isinstance(result, dict):
            return result.get("tree", "")
        return result

    # --- Proxy execution ---

    def proxy_exec(
        self,
        *command: str,
        cwd: str = "/",
        readonly: bool = False,
        timeout: Optional[int] = None,
        checkpoint: bool = True,
    ) -> Dict[str, Any]:
        """Execute a command inside a mounted workspace.

        Args:
            command: Command and arguments to execute.
            cwd: Working directory inside the mounted vault.
            readonly: Mount the vault read-only.
            timeout: Timeout in milliseconds (default: 300000).
            checkpoint: Create an auto-checkpoint before execution.

        Returns:
            Structured result with stdout, stderr, exit_code, changed_files.
        """
        args = ["proxy", "exec"]
        if readonly:
            args.append("--readonly")
        if cwd != "/":
            args.extend(["--cwd", cwd])
        if timeout is not None:
            args.extend(["--timeout", str(timeout)])
        if not checkpoint:
            # There is no --no-checkpoint flag; this is a design note.
            pass
        args.append("--")
        args.extend(command)
        return self._run(*args)  # type: ignore

    # --- Checkpoints ---

    def checkpoint_save(self, name: str) -> Dict[str, Any]:
        """Save a checkpoint."""
        return self._run("checkpoint", "save", name)  # type: ignore

    def checkpoint_restore(self, name: str) -> Dict[str, Any]:
        """Restore a checkpoint."""
        return self._run("checkpoint", "restore", name)  # type: ignore

    def checkpoint_list(self) -> List[Dict[str, Any]]:
        """List checkpoints."""
        result = self._run("checkpoint", "list")
        if isinstance(result, dict):
            return result.get("checkpoints", [])
        return []

    # --- Search ---

    def grep(self, pattern: str, path: str = "/") -> List[Dict[str, Any]]:
        """Search file contents with regex."""
        result = self._run("grep", pattern, path)
        if isinstance(result, dict):
            return result.get("matches", [])
        return []

    def search(self, query: str, path: Optional[str] = None) -> List[Dict[str, Any]]:
        """Full-text search."""
        args = ["search", query]
        if path:
            args.append(path)
        result = self._run(*args)
        if isinstance(result, dict):
            return result.get("results", [])
        return []

    # --- Maintenance ---

    def stats(self) -> Dict[str, Any]:
        """Show vault statistics."""
        return self._run("stats")  # type: ignore

    def gc(self) -> Dict[str, Any]:
        """Run garbage collection."""
        return self._run("gc")  # type: ignore

    def compact(self) -> Dict[str, Any]:
        """Compact the database."""
        return self._run("compact")  # type: ignore
