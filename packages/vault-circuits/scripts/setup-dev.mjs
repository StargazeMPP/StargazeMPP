#!/usr/bin/env node
/**
 * Dev-only Phase-2 setup for the `aggregate_sum` circuit.
 *
 * Produces local artifacts under `build/aggregate_sum_*` suitable for
 * unit tests and local development. The contribution entropy is hard-
 * coded — do NOT use the resulting `.zkey` in production; the real
 * ceremony is coordinated separately (see `docs/vault-ceremony.md`).
 *
 * Shells out to the `snarkjs` CLI rather than its JS API — the API
 * surface drifts between minor versions, the CLI is stable, and the
 * docs uniformly use it.
 *
 * Run: `npm run setup:aggregate-dev`
 */

import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, unlinkSync } from 'node:fs';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const root = path.resolve(__dirname, '..');
const buildDir = path.join(root, 'build');
const circuit = 'aggregate_sum';

mkdirSync(buildDir, { recursive: true });

const r1cs = path.join(buildDir, `${circuit}.r1cs`);
if (!existsSync(r1cs)) {
  throw new Error(`Missing ${r1cs} — run \`npm run compile:aggregate\` first.`);
}

const pot0 = path.join(buildDir, 'pot12_0000.ptau');
const pot1 = path.join(buildDir, 'pot12_0001.ptau');
const potFinal = path.join(buildDir, 'pot12_final.ptau');
const zkey0 = path.join(buildDir, `${circuit}_0000.zkey`);
const zkeyFinal = path.join(buildDir, `${circuit}_final.zkey`);
const vkeyJson = path.join(buildDir, `${circuit}_vkey.json`);

function run(args) {
  console.log('$ npx snarkjs', args.join(' '));
  execFileSync('npx', ['--yes', 'snarkjs', ...args], { stdio: 'inherit', cwd: root });
}

console.log('[1/5] powers of tau new');
run(['powersoftau', 'new', 'bn128', '12', pot0, '-v']);

console.log('[2/5] powers of tau contribute');
run([
  'powersoftau',
  'contribute',
  pot0,
  pot1,
  '--name=stargaze-dev',
  '-v',
  '-e=dev-entropy-do-not-use-in-prod',
]);

console.log('[3/5] powers of tau prepare phase 2');
run(['powersoftau', 'prepare', 'phase2', pot1, potFinal, '-v']);

console.log(`[4/5] groth16 setup for ${circuit}`);
run(['groth16', 'setup', r1cs, potFinal, zkey0]);
run([
  'zkey',
  'contribute',
  zkey0,
  zkeyFinal,
  '--name=stargaze-dev',
  '-v',
  '-e=dev-entropy-do-not-use-in-prod',
]);

console.log('[5/5] export verifying key');
run(['zkey', 'export', 'verificationkey', zkeyFinal, vkeyJson]);

for (const f of [pot0, pot1, zkey0]) {
  try {
    unlinkSync(f);
  } catch {
    /* ignore */
  }
}

console.log('\nDone.');
console.log('  zkey:', zkeyFinal);
console.log('  vkey:', vkeyJson);
