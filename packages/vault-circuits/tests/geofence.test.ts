import { describe, it, expect, beforeAll } from 'vitest';
import path from 'node:path';
import { existsSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { groth16 } from 'snarkjs';
import { SnarkjsVaultProofGenerator } from '../src/ts/proof-generator.js';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const buildDir = path.resolve(__dirname, '..', 'build');

const wasm = path.join(buildDir, 'geofence_js', 'geofence.wasm');
const zkey = path.join(buildDir, 'geofence_final.zkey');
const vkey = path.join(buildDir, 'geofence_vkey.json');

const hasArtifacts = existsSync(wasm) && existsSync(zkey) && existsSync(vkey);

// 2^31 offset so signed micro-degrees encode as unsigned < 2^32.
const OFFSET = 2_147_483_648n;
function encode(degrees: number): bigint {
  return BigInt(Math.round(degrees * 1_000_000)) + OFFSET;
}

// Box: lat ∈ [40°, 41°], lon ∈ [-74°, -73°] (roughly NYC).
const minLat = encode(40);
const maxLat = encode(41);
const minLon = encode(-74);
const maxLon = encode(-73);

describe.skipIf(!hasArtifacts)('Geofence circuit', () => {
  let generator: SnarkjsVaultProofGenerator;

  beforeAll(() => {
    generator = new SnarkjsVaultProofGenerator({
      'geofence/v1': { wasm, zkey, verifyingKeyJsonPath: vkey },
    });
  });

  it('proves and verifies a point inside the box', async () => {
    // Times Square: lat 40.7580, lon -73.9855 — comfortably inside [40,41] × [-74,-73].
    const lat = encode(40.758);
    const lon = encode(-73.9855);

    const output = await generator.generate('geofence/v1', {
      privateInputs: { lat, lon },
      publicParams: { minLat, maxLat, minLon, maxLon },
    });

    expect(output.publicOutput).toHaveLength(4);
    expect(output.proofHash).toMatch(/^0x[0-9a-f]{64}$/);

    const { proof } = await groth16.fullProve(
      { lat, lon, minLat, maxLat, minLon, maxLon },
      wasm,
      zkey,
    );
    const { readFileSync } = await import('node:fs');
    const vk = JSON.parse(readFileSync(vkey, 'utf-8'));
    const ok = await groth16.verify(
      vk,
      [minLat.toString(), maxLat.toString(), minLon.toString(), maxLon.toString()],
      proof,
    );
    expect(ok).toBe(true);
  });

  it('rejects a point outside the box', async () => {
    // London is way outside the NYC box.
    const lat = encode(51.5072);
    const lon = encode(-0.1276);

    await expect(
      generator.generate('geofence/v1', {
        privateInputs: { lat, lon },
        publicParams: { minLat, maxLat, minLon, maxLon },
      }),
    ).rejects.toThrow();
  });
});
