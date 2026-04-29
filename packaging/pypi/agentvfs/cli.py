"""Console-script entry point for the `agentvfs` command.

Resolves the native `avfs` binary in this priority order:

  1. AGENTVFS_BIN env var (explicit override — useful for CI / dev builds)
  2. `avfs` on PATH (e.g. user already installed via cargo / install.sh / npm)
  3. Cached binary at $XDG_CACHE_HOME/agentvfs/<version>/avfs
  4. Lazy first-run download from the GitHub release matching this package's
     version, extracted into the cache directory, then exec'd.

Cache layout:
    $XDG_CACHE_HOME/agentvfs/<version>/avfs        (or avfs.exe on Windows)

To force a re-download (e.g. corrupted cache):
    rm -rf ~/.cache/agentvfs

Implementation notes:
  - The cache key includes the package version, so `pip install -U
    agentvfs-cli` automatically triggers a fresh download for the new
    release (older version dirs sit alongside, harmless).
  - We deliberately do NOT register `avfs` as a console-script in
    pyproject.toml — that would create a Python shim named `avfs` which
    shutil.which() in step 2 could resolve to itself, recursing forever.
"""

from __future__ import annotations

import io
import os
import platform
import shutil
import sys
import tarfile
import tempfile
import urllib.error
import urllib.request
import zipfile
from importlib.metadata import PackageNotFoundError, version as _pkg_version
from pathlib import Path

REPO = "neul-labs/agentvfs"
DIST_NAME = "agentvfs-cli"

# (platform.system(), platform.machine()) -> (release archive slug, extension).
# Keep this aligned with the build matrix in .github/workflows/release.yml.
_PLATFORM_MAP: dict[tuple[str, str], tuple[str, str]] = {
    ("Linux",   "x86_64"):  ("linux-x86_64",   "tar.gz"),
    ("Linux",   "aarch64"): ("linux-aarch64",  "tar.gz"),
    ("Linux",   "arm64"):   ("linux-aarch64",  "tar.gz"),
    ("Darwin",  "x86_64"):  ("darwin-x86_64",  "tar.gz"),
    ("Darwin",  "arm64"):   ("darwin-aarch64", "tar.gz"),
    ("Windows", "AMD64"):   ("windows-x86_64", "zip"),
    ("Windows", "x86_64"):  ("windows-x86_64", "zip"),
}


def _binary_name() -> str:
    return "avfs.exe" if os.name == "nt" else "avfs"


def _package_version() -> str:
    try:
        return _pkg_version(DIST_NAME)
    except PackageNotFoundError:
        # Running from a source checkout without an installed dist (e.g. tests).
        return "0.0.0"


def _cache_dir() -> Path:
    base = os.environ.get("XDG_CACHE_HOME") or str(Path.home() / ".cache")
    return Path(base) / "agentvfs" / _package_version()


def _platform_release() -> tuple[str, str]:
    key = (platform.system(), platform.machine())
    if key not in _PLATFORM_MAP:
        supported = ", ".join("/".join(k) for k in _PLATFORM_MAP)
        raise RuntimeError(
            f"Unsupported platform: {key[0]} / {key[1]}.\n"
            f"Supported: {supported}\n"
            f"Install manually: https://github.com/{REPO}/releases"
        )
    return _PLATFORM_MAP[key]


def _download_binary(cache_dir: Path) -> Path:
    version = _package_version()
    slug, ext = _platform_release()
    archive = f"avfs-{version}-{slug}.{ext}"
    url = f"https://github.com/{REPO}/releases/download/v{version}/{archive}"

    sys.stderr.write(f"agentvfs: fetching native binary {archive}\n")
    sys.stderr.write(f"agentvfs: from {url}\n")

    cache_dir.mkdir(parents=True, exist_ok=True)
    req = urllib.request.Request(url, headers={"User-Agent": f"{DIST_NAME}/{version}"})
    try:
        with urllib.request.urlopen(req) as r:
            data = r.read()
    except urllib.error.HTTPError as e:
        raise RuntimeError(
            f"Download failed: HTTP {e.code} {e.reason} for {url}\n"
            f"Either the GitHub release v{version} is not yet published, "
            f"or no archive exists for this platform.\n"
            f"Install manually: https://github.com/{REPO}/releases"
        ) from e
    except urllib.error.URLError as e:
        raise RuntimeError(
            f"Download failed: {e.reason} for {url}\n"
            f"Check your network connection, or install manually:\n"
            f"  https://github.com/{REPO}/releases"
        ) from e

    # Extract into a temp dir inside the cache, then atomically move the binary
    # into its final location. Two parallel first-runs are safe: each extracts
    # into its own tmp dir; whichever os.replace lands last wins, which is fine
    # because the contents are bit-identical.
    buf = io.BytesIO(data)
    with tempfile.TemporaryDirectory(prefix="dl-", dir=cache_dir) as tmp:
        tmp_path = Path(tmp)
        if ext == "zip":
            with zipfile.ZipFile(buf) as zf:
                zf.extractall(tmp_path)
        else:
            with tarfile.open(fileobj=buf, mode="r:gz") as tf:
                tf.extractall(tmp_path)

        src = tmp_path / _binary_name()
        if not src.is_file():
            raise RuntimeError(
                f"Archive does not contain {_binary_name()}: extracted to {tmp_path}"
            )
        src.chmod(0o755)
        dest = cache_dir / _binary_name()
        os.replace(src, dest)

    sys.stderr.write(f"agentvfs: cached at {dest}\n")
    return dest


def _resolve_binary() -> Path:
    override = os.environ.get("AGENTVFS_BIN")
    if override:
        p = Path(override).expanduser()
        if not p.is_file():
            raise RuntimeError(
                f"AGENTVFS_BIN points to a non-existent file: {override}"
            )
        return p

    on_path = shutil.which("avfs")
    if on_path:
        return Path(on_path)

    cache_dir = _cache_dir()
    cached = cache_dir / _binary_name()
    if cached.is_file():
        return cached

    return _download_binary(cache_dir)


def main() -> int:
    try:
        binary = _resolve_binary()
    except Exception as e:
        sys.stderr.write(f"agentvfs: {e}\n")
        return 1

    # On POSIX, execvp replaces this process so signals/exit codes propagate
    # cleanly. On Windows os.execvp spawns a child and exits the parent,
    # which loses signal forwarding — fall back to subprocess there.
    if os.name == "nt":
        import subprocess

        return subprocess.run([str(binary), *sys.argv[1:]]).returncode

    os.execvp(str(binary), [str(binary), *sys.argv[1:]])
    return 1  # unreachable on success


if __name__ == "__main__":
    sys.exit(main() or 0)
