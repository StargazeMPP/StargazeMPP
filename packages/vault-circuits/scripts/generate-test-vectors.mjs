#!/usr/bin/env node
/**
 * Generates a Groth16 proof + public signals for each circuit using known
 * inputs, then writes them as JSON test vectors under
 * `packages/contracts-evm/test/vectors/<circuit>.json` for the Foundry
 * verifier tests to consume.
 *
 * The committed verifier source under `packages/contracts-evm/src/verifiers/`
 * is locked to the current `<circuit>_final.zkey` — regenerate both via
 * `npm run all:<circuit>-dev && node scripts/generate-test-vectors.mjs`
 * whenever the zkey changes.
 */

import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { mkdirSync, writeFileSync } from 'node:fs';
import { groth16 } from 'snarkjs';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(__dirname, '..');
const buildDir = path.join(root, 'build');
const outDir = path.resolve(root, '..', 'contracts-evm', 'test', 'vectors');

mkdirSync(outDir, { recursive: true });

const CIRCUITS = [
  {
    name: 'aggregate_sum',
    // Inputs cohere with the comment-block summary in the .circom: sum 1..8 = 36.
    input: {
      values: [1, 2, 3, 4, 5, 6, 7, 8].map((v) => v.toString()),
      claimedSum: '36',
    },
  },
  {
    name: 'aggregate_mean',
    // 3+5+...+17 = 80, mean 80/8 = 10 — exact integer.
    input: {
      values: [3, 5, 7, 9, 11, 13, 15, 17].map((v) => v.toString()),
      claimedMean: '10',
    },
  },
  {
    name: 'geofence',
    // Encoded micro-degree with a 2^31 offset (see geofence.circom for the
    // convention). Box is roughly Berlin, point is the Brandenburg Gate.
    // All values fit comfortably under 2^32.
    input: (() => {
      const offset = 1n << 31n; // 2^31
      const enc = (deg) => (BigInt(Math.round(deg * 1_000_000)) + offset).toString();
      return {
        lat: enc(52.5163),
        lon: enc(13.3777),
        minLat: enc(52.3),
        maxLat: enc(52.7),
        minLon: enc(13.1),
        maxLon: enc(13.6),
      };
    })(),
  },
];

for (const { name, input } of CIRCUITS) {
  const wasm = path.join(buildDir, `${name}_js`, `${name}.wasm`);
  const zkey = path.join(buildDir, `${name}_final.zkey`);

  const { proof, publicSignals } = await groth16.fullProve(input, wasm, zkey);
  const calldataStr = await groth16.exportSolidityCallData(proof, publicSignals);
  const parsed = JSON.parse(`[${calldataStr}]`);
  const [a, b, c, pubSignals] = parsed;

  const vector = {
    circuit: name,
    input,
    a,
    b,
    c,
    pubSignals,
  };
  const outFile = path.join(outDir, `${name}.json`);
  writeFileSync(outFile, JSON.stringify(vector, null, 2) + '\n', 'utf-8');
  console.log(`  ✓ ${name} → packages/contracts-evm/test/vectors/${name}.json (${pubSignals.length} public signal(s))`);
}
