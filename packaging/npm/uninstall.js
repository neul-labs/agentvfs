#!/usr/bin/env node
const fs = require('fs');
const path = require('path');

const binDir = path.join(__dirname, 'bin');
if (fs.existsSync(binDir)) {
  fs.rmSync(binDir, { recursive: true, force: true });
  console.log('Removed avfs binaries');
}
