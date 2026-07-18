#!/usr/bin/env node
import { spawnSync } from 'child_process';
import { fileURLToPath } from 'url';
import { dirname, resolve } from 'path';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

const frontendDir = resolve(__dirname, '..');
const indexFile = resolve(frontendDir, 'src', 'index.tsx');

const result = spawnSync(
  'npx',
  ['--no-install', 'tsx', indexFile, ...process.argv.slice(2)],
  {
    cwd: frontendDir,
    stdio: 'inherit',
    env: {
      ...process.env,
      FORCE_COLOR: '1'
    }
  }
);

process.exit(result.status ?? 0);
