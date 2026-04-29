"""AgentVFS Python wrapper.

A thin Python wrapper around the avfs CLI for agent integration.
"""

from importlib.metadata import PackageNotFoundError, version as _pkg_version

try:
    __version__ = _pkg_version("agentvfs-cli")
except PackageNotFoundError:
    __version__ = "0.0.0"

from .client import AVFS, AVFSError, VaultNotFoundError

__all__ = ["AVFS", "AVFSError", "VaultNotFoundError", "__version__"]
