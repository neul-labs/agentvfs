#!/usr/bin/env node
const fs = require('fs');
const path = require('path');
const https = require('https');
const { execSync } = require('child_process');

const REPO = 'neul-labs/agentvfs';
const BINARY_NAME = 'avfs';

function getPlatform() {
  const platform = process.platform;
  const arch = process.arch;

  const map = {
    'linux-x64': 'linux-x86_64',
    'linux-arm64': 'linux-aarch64',
    'darwin-x64': 'darwin-x86_64',
    'darwin-arm64': 'darwin-aarch64',
    'win32-x64': 'windows-x86_64',
  };

  const key = `${platform}-${arch}`;
  const target = map[key];
  if (!target) {
    console.error(`Unsupported platform: ${key}`);
    console.error('Supported platforms: linux-x86_64, linux-aarch64, darwin-x86_64, darwin-aarch64, windows-x86_64');
    process.exit(1);
  }
  return target;
}

function getVersion() {
  const pkg = JSON.parse(fs.readFileSync(path.join(__dirname, 'package.json'), 'utf8'));
  return pkg.version;
}

function getExtension() {
  return process.platform === 'win32' ? 'zip' : 'tar.gz';
}

function getBinaryName() {
  return process.platform === 'win32' ? `${BINARY_NAME}.exe` : BINARY_NAME;
}

async function download(url, dest) {
  return new Promise((resolve, reject) => {
    const file = fs.createWriteStream(dest);
    https.get(url, (response) => {
      if (response.statusCode === 302 || response.statusCode === 301) {
        download(response.headers.location, dest).then(resolve).catch(reject);
        return;
      }
      if (response.statusCode !== 200) {
        reject(new Error(`Download failed: ${response.statusCode}`));
        return;
      }
      response.pipe(file);
      file.on('finish', () => {
        file.close(resolve);
      });
    }).on('error', (err) => {
      fs.unlink(dest, () => {});
      reject(err);
    });
  });
}

function extract(archivePath, extractDir) {
  if (archivePath.endsWith('.zip')) {
    execSync(`unzip -o "${archivePath}" -d "${extractDir}"`, { stdio: 'inherit' });
  } else {
    execSync(`tar -xzf "${archivePath}" -C "${extractDir}"`, { stdio: 'inherit' });
  }
}

async function main() {
  const version = getVersion();
  const platform = getPlatform();
  const ext = getExtension();
  const archiveName = `${BINARY_NAME}-${version}-${platform}.${ext}`;
  const url = `https://github.com/${REPO}/releases/download/v${version}/${archiveName}`;

  const binDir = path.join(__dirname, 'bin');
  const archivePath = path.join(binDir, archiveName);

  fs.mkdirSync(binDir, { recursive: true });

  console.log(`Downloading avfs v${version} for ${platform}...`);
  console.log(`URL: ${url}`);

  try {
    await download(url, archivePath);
  } catch (err) {
    console.error(`Failed to download: ${err.message}`);
    console.error('Falling back to building from source with cargo...');
    try {
      execSync('cargo install agentvfs', { stdio: 'inherit' });
      console.log('Installed via cargo. Ensure ~/.cargo/bin is in your PATH.');
      return;
    } catch {
      console.error('Cargo install also failed. Please install manually.');
      process.exit(1);
    }
  }

  console.log('Extracting...');
  extract(archivePath, binDir);

  const binaryPath = path.join(binDir, getBinaryName());
  if (fs.existsSync(binaryPath)) {
    fs.chmodSync(binaryPath, 0o755);
    console.log(`Installed avfs to ${binaryPath}`);
  } else {
    console.error('Binary not found after extraction');
    process.exit(1);
  }

  fs.unlinkSync(archivePath);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
