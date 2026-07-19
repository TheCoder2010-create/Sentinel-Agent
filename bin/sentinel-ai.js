#!/usr/bin/env node
import { spawn, execSync } from 'child_process';
import { existsSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

const PLATFORM = process.platform;   // win32, linux, darwin
const ARCH = process.arch;           // x64, arm64

function binaryName() {
  const base = 'sentinel';
  return PLATFORM === 'win32' ? `${base}.exe` : base;
}

function searchPaths() {
  const name = binaryName();
  const paths = [];

  // Next to this script
  paths.push(resolve(__dirname, name));
  // Project target directory (dev layout)
  paths.push(resolve(__dirname, '..', 'target', 'release', name));
  paths.push(resolve(__dirname, '..', 'target', 'debug', name));
  // npm global prefix
  try {
    const prefix = execSync('npm prefix -g', { encoding: 'utf8' }).trim();
    paths.push(resolve(prefix, 'lib', 'node_modules', 'sentinel-ai', 'bin', name));
  } catch { /* ignore */ }
  // PATH
  return paths;
}

function findBinary() {
  for (const p of searchPaths()) {
    if (existsSync(p)) return p;
  }
  return null;
}

function reinstallHint() {
  const lines = [
    '╔══════════════════════════════════════════════════════════╗',
    '║  Sentinel AI native binary not found                    ║',
    '╠══════════════════════════════════════════════════════════╣',
    '║                                                         ║',
    '║  Build the CLI with:                                    ║',
    '║                                                         ║',
    '║    cargo build --release                                ║',
    '║                                                         ║',
    '║  Or install via:                                        ║',
    '║                                                         ║',
    '║    cargo install --path crates/sentinel-cli             ║',
    '║                                                         ║',
    '╚══════════════════════════════════════════════════════════╝',
  ];
  return lines.join('\n');
}

const binary = findBinary();
if (!binary) {
  console.error(reinstallHint());
  process.exit(1);
}

const args = process.argv.slice(2);
const child = spawn(binary, args, { stdio: 'inherit', shell: false });

child.on('exit', (code, signal) => {
  if (signal) process.kill(process.pid, signal);
  else process.exit(code ?? 0);
});

child.on('error', (err) => {
  console.error(`Failed to launch sentinel: ${err.message}`);
  process.exit(1);
});
