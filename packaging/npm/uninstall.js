#!/usr/bin/env node
// Clean up the postinstall-downloaded native binary. The JS shim in bin/
// is part of the package itself — npm removes it as part of normal uninstall.
const fs = require('fs');
const path = require('path');

const vendorDir = path.join(__dirname, 'vendor');
if (fs.existsSync(vendorDir)) {
  fs.rmSync(vendorDir, { recursive: true, force: true });
  console.log('Removed avfs native binary');
}
