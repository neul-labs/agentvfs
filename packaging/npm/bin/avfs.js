#!/usr/bin/env node
'use strict';

// Thin shim: forwards argv/stdio to the native avfs binary that postinstall
// downloads into ../vendor/. This file is what npm symlinks as `avfs` /
// `agentvfs` on $PATH, so it must always exist in the published tarball
// (the actual binary is platform-specific and only present after install).

const fs = require('fs');
const path = require('path');
const { spawnSync } = require('child_process');

const binaryName = process.platform === 'win32' ? 'avfs.exe' : 'avfs';
const binaryPath = path.join(__dirname, '..', 'vendor', binaryName);

if (!fs.existsSync(binaryPath)) {
  console.error(`avfs binary not found at ${binaryPath}`);
  console.error('The postinstall step may have failed.');
  console.error('Try: npm rebuild agentvfs   (or reinstall the package)');
  process.exit(1);
}

const result = spawnSync(binaryPath, process.argv.slice(2), {
  stdio: 'inherit',
  windowsHide: true,
});

if (result.error) {
  console.error(result.error);
  process.exit(1);
}

process.exit(result.status === null ? 1 : result.status);
