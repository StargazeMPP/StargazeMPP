#!/usr/bin/env node
// Extracts ABI + bytecode hash from `forge build` artifacts and writes them
// into `packages/shared/src/evm/abi/` so the backend can import them as a
// typed boundary contract.
//
// Run after `forge build`. Idempotent.

import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createHash } from 'node:crypto';

const __dirname = dirname(fileURLToPath(import.meta.url));
const root = resolve(__dirname, '..');
const outDir = resolve(root, 'out');
const sharedAbiDir = resolve(root, '..', 'shared', 'src', 'evm', 'abi');

const CONTRACTS = [
  'GAZEToken',
  'BurnController',
  'StargazeEscrow',
  'StargazeRegistry',
  'PrivacyVaultRegistry',
];

mkdirSync(sharedAbiDir, { recursive: true });

const manifest = {
  generatedAt: new Date().toISOString(),
  contracts: {},
};

for (const name of CONTRACTS) {
  const artifactPath = resolve(outDir, `${name}.sol`, `${name}.json`);
  const artifact = JSON.parse(readFileSync(artifactPath, 'utf8'));
  const slim = {
    contractName: name,
    abi: artifact.abi,
    bytecodeHash: createHash('sha256').update(artifact.bytecode?.object ?? '').digest('hex'),
    deployedBytecodeHash: createHash('sha256').update(artifact.deployedBytecode?.object ?? '').digest('hex'),
  };
  const outFile = resolve(sharedAbiDir, `${name}.json`);
  writeFileSync(outFile, JSON.stringify(slim, null, 2) + '\n', 'utf8');
  manifest.contracts[name] = {
    file: `./${name}.json`,
    bytecodeHash: slim.bytecodeHash,
  };
  console.log(`  ✓ ${name} → packages/shared/src/evm/abi/${name}.json`);
}

writeFileSync(resolve(sharedAbiDir, 'manifest.json'), JSON.stringify(manifest, null, 2) + '\n', 'utf8');
console.log(`\nWrote manifest with ${CONTRACTS.length} contracts.`);
