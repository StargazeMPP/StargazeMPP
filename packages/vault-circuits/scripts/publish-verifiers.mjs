#!/usr/bin/env node
/**
 * Copies the snarkjs-generated `build/<Circuit>Verifier.sol` files into
 * `packages/contracts-evm/src/verifiers/`, renaming the `Groth16Verifier`
 * wrapper contract to a per-circuit name so multiple verifiers can coexist
 * in one project.
 *
 * Re-run whenever the circuits' zkeys change. The output files are checked
 * in alongside the rest of the contract source.
 */

import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(__dirname, '..');
const buildDir = path.join(root, 'build');
const outDir = path.resolve(root, '..', 'contracts-evm', 'src', 'verifiers');

const VERIFIERS = [
  { source: 'AggregateSumVerifier.sol', contractName: 'AggregateSumVerifier' },
  { source: 'GeofenceVerifier.sol', contractName: 'GeofenceVerifier' },
];

mkdirSync(outDir, { recursive: true });

for (const { source, contractName } of VERIFIERS) {
  const src = path.join(buildDir, source);
  const raw = readFileSync(src, 'utf-8');
  // Tighten pragma to the rest of the project (0.8.27) and rename the
  // single wrapper contract — snarkjs emits everything as `Groth16Verifier`.
  const rewritten = raw
    .replace(/pragma solidity[^;]+;/, 'pragma solidity ^0.8.20;')
    .replace(/contract Groth16Verifier\b/, `contract ${contractName}`);
  const dest = path.join(outDir, source);
  writeFileSync(dest, rewritten, 'utf-8');
  console.log(`  ✓ ${source} → packages/contracts-evm/src/verifiers/${source} (contract ${contractName})`);
}
