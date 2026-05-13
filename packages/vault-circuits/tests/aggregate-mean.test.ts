import { describe, it, expect, beforeAll } from 'vitest';
import path from 'node:path';
import { existsSync, readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { groth16 } from 'snarkjs';
import { SnarkjsVaultProofGenerator } from '../src/ts/proof-generator.js';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const buildDir = path.resolve(__dirname, '..', 'build');

const wasm = path.join(buildDir, 'aggregate_mean_js', 'aggregate_mean.wasm');
const zkey = path.join(buildDir, 'aggregate_mean_final.zkey');
const vkey = path.join(buildDir, 'aggregate_mean_vkey.json');

const hasArtifacts = existsSync(wasm) && existsSync(zkey) && existsSync(vkey);

describe.skipIf(!hasArtifacts)('AggregateMean', () => {
  let generator: SnarkjsVaultProofGenerator;

  beforeAll(() => {
    generator = new SnarkjsVaultProofGenerator({
      'aggregate-mean/v1': { wasm, zkey, verifyingKeyJsonPath: vkey },
    });
  });

  it('proves and verifies the integer mean of eight private values', async () => {
    // Pick eight values whose sum is divisible by 8 so the integer mean
    // matches the true mean exactly: 3+5+...+17 = 80, 80/8 = 10.
    const values = [3, 5, 7, 9, 11, 13, 15, 17].map((v) => BigInt(v));
    const claimedMean = 10n;

    const output = await generator.generate('aggregate-mean/v1', {
      privateInputs: { values },
      publicParams: { claimedMean },
    });

    expect(output.publicOutput).toHaveLength(1);
    expect(BigInt(output.publicOutput[0] as string)).toBe(claimedMean);
    expect(output.proofHash).toMatch(/^0x[0-9a-f]{64}$/);

    const { proof } = await groth16.fullProve({ values, claimedMean }, wasm, zkey);
    const directOk = await groth16.verify(
      JSON.parse(readFileSync(vkey, 'utf-8')),
      [claimedMean.toString()],
      proof,
    );
    expect(directOk).toBe(true);
  });

  it('rejects an off-by-one claimed mean', async () => {
    // sum=80, claimedMean=11 implies 11*8=88 != 80 → witness fails.
    const values = [3, 5, 7, 9, 11, 13, 15, 17].map((v) => BigInt(v));
    const claimedMean = 11n;

    await expect(
      groth16.fullProve({ values, claimedMean }, wasm, zkey),
    ).rejects.toThrow();
  });

  it('handles a cohort where the integer mean is the floor of the true mean', async () => {
    // sum = 39, true mean 4.875, but only integer means satisfy the
    // circuit. The publisher must pick a claimedMean such that
    // N * claimedMean == sum exactly. There is no claimedMean that
    // works for sum=39, N=8 — assert the witness generation fails for
    // both floor(4) and ceil(5).
    const values = [1, 2, 3, 5, 5, 6, 7, 10].map((v) => BigInt(v));

    await expect(
      groth16.fullProve({ values, claimedMean: 4n }, wasm, zkey),
    ).rejects.toThrow();
    await expect(
      groth16.fullProve({ values, claimedMean: 5n }, wasm, zkey),
    ).rejects.toThrow();
  });
});
