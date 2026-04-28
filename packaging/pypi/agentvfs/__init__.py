"""AgentVFS Python wrapper.

A thin Python wrapper around the avfs CLI for agent integration.
"""

__version__ = "0.1.0"

from .client import AVFS, AVFSError, VaultNotFoundError

__all__ = ["AVFS", "AVFSError", "VaultNotFoundError"]
