# agentvfs

Cross-platform npm wrapper for the `avfs` CLI.

This package downloads the correct platform-specific binary from GitHub releases during installation.

## Install

```bash
npm install -g agentvfs
```

## Usage

```bash
avfs vault create myproject
avfs proxy exec -- python script.py
```

## Supported Platforms

- Linux x86_64 / ARM64
- macOS x86_64 / Apple Silicon
- Windows x86_64

## Fallback

If the GitHub release download fails, the installer falls back to `cargo install agentvfs`.
