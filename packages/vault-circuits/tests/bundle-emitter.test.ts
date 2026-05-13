import { describe, it, expect } from 'vitest';
import path from 'node:path';
import { existsSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

// @ts-expect-error — emit-rust-vkey.mjs is a JS module without types.
import { buildBundle, proveAndEncode } from '../scripts/emit-rust-vkey.mjs';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const buildDir = path.resolve(__dirname, '..', 'build');

const wasm = path.join(buildDir, 'aggregate_sum_js', 'aggregate_sum.wasm');
const zkey = path.join(buildDir, 'aggregate_sum_final.zkey');
const hasArtifacts = existsSync(wasm) && existsSync(zkey);

interface ProofBundle {
  proofHex: string;
  publicSignalsHex: string[];
}

describe.skipIf(!hasArtifacts)('buildBundle (aggregate_sum)', () => {
  const inputs = { values: [1, 2, 3, 4, 5, 6, 7, 8], claimedSum: 36 };

  it('emits a JSON bundle with the schema the submit-vault-proof CLI consumes', async () => {
    const json = await buildBundle('aggregate_sum', inputs);
    expect(typeof json).toBe('string');
    expect(json.endsWith('\n')).toBe(true);

    const parsed = JSON.parse(json) as ProofBundle;
    expect(typeof parsed.proofHex).toBe('string');
    expect(Array.isArray(parsed.publicSignalsHex)).toBe(true);

    // 256-byte proof = 512 hex chars, no 0x prefix.
    expect(parsed.proofHex).toMatch(/^[0-9a-f]{512}$/);

    // aggregate_sum has exactly one public signal (claimedSum).
    expect(parsed.publicSignalsHex).toHaveLength(1);
    expect(parsed.publicSignalsHex[0]).toMatch(/^[0-9a-f]{64}$/);

    // Public signal is the big-endian encoding of claimedSum = 36.
    expect(BigInt('0x' + parsed.publicSignalsHex[0])).toBe(36n);
  });

  it('keys match what proveAndEncode returns when re-hex-encoded', async () => {
    // Two distinct calls produce two distinct proofs (snarkjs adds
    // randomness), but signal bytes are derived from public inputs and
    // should round-trip 1:1 between the byte form and the hex form.
    const { signalBytes } = (await proveAndEncode('aggregate_sum', inputs)) as {
      proofBytes: Uint8Array;
      signalBytes: Uint8Array[];
      publicSignals: string[];
    };
    const json = await buildBundle('aggregate_sum', inputs);
    const parsed = JSON.parse(json) as ProofBundle;

    expect(parsed.publicSignalsHex).toHaveLength(signalBytes.length);
    for (let i = 0; i < signalBytes.length; i++) {
      const decoded = hexToBytes(parsed.publicSignalsHex[i]);
      expect(Array.from(decoded)).toEqual(Array.from(signalBytes[i]));
    }
  });
});

describe('buildBundle (no artifacts)', () => {
  it('throws a useful error when the circuit build is missing', async () => {
    await expect(buildBundle('definitely-not-a-circuit', {})).rejects.toThrow(
      /Missing artifact/,
    );
  });
});

function hexToBytes(hex: string): Uint8Array {
  const stripped = hex.startsWith('0x') ? hex.slice(2) : hex;
  if (stripped.length % 2 !== 0) {
    throw new Error('hex string must have an even length');
  }
  const out = new Uint8Array(stripped.length / 2);
  for (let i = 0; i < out.length; i++) {
    out[i] = parseInt(stripped.slice(i * 2, i * 2 + 2), 16);
  }
  return out;
}
