import { describe, expect, it } from 'vitest';
import { mkdtempSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import {
  decodeHex,
  loadKeypair,
  loadProofBundle,
  normaliseHex,
  parseArgs,
} from './submit-vault-proof.js';

function tmpFile(name: string, contents: string): string {
  const dir = mkdtempSync(join(tmpdir(), 'stargaze-submit-proof-'));
  const path = join(dir, name);
  writeFileSync(path, contents);
  return path;
}

const VALID_ARGS = [
  '--keypair', '/tmp/kp.json',
  '--verifier', 'CTC7ehb1sYj7A5EsAd3E6viYdo5bxydzSpccDENbkUmP',
  '--provider-id', 'aa'.repeat(32),
  '--proof', '/tmp/bundle.json',
];

describe('parseArgs', () => {
  it('returns defaults for optional flags', () => {
    const args = parseArgs(VALID_ARGS);
    expect(args.rpc).toBe('https://api.devnet.solana.com');
    expect(args.computeUnits).toBe(600_000);
    expect(args.commitment).toBe('confirmed');
    expect(args.dryRun).toBe(false);
    expect(args.stargazeProgramId).toBeUndefined();
  });

  it('threads through overrides', () => {
    const args = parseArgs([
      ...VALID_ARGS,
      '--rpc', 'http://localhost:8899',
      '--compute-units', '400000',
      '--commitment', 'finalized',
      '--stargaze-program-id', 'Stake11111111111111111111111111111111111111',
      '--dry-run',
    ]);
    expect(args.rpc).toBe('http://localhost:8899');
    expect(args.computeUnits).toBe(400_000);
    expect(args.commitment).toBe('finalized');
    expect(args.stargazeProgramId).toBe('Stake11111111111111111111111111111111111111');
    expect(args.dryRun).toBe(true);
  });

  it('throws on missing required flag', () => {
    expect(() => parseArgs(['--keypair', '/tmp/kp.json'])).toThrow(/missing required flag/);
  });

  it('throws on unknown flag', () => {
    expect(() => parseArgs([...VALID_ARGS, '--bogus', 'x'])).toThrow(/unknown flag/);
  });

  it('throws on missing flag value', () => {
    expect(() => parseArgs([...VALID_ARGS, '--rpc'])).toThrow(/requires a value/);
  });

  it('rejects a flag value that looks like another flag', () => {
    expect(() => parseArgs(['--keypair', '--verifier'])).toThrow(/requires a value/);
  });

  it('rejects non-positive compute-units', () => {
    expect(() => parseArgs([...VALID_ARGS, '--compute-units', '0'])).toThrow(/positive integer/);
    expect(() => parseArgs([...VALID_ARGS, '--compute-units', 'abc'])).toThrow(/positive integer/);
  });
});

describe('normaliseHex', () => {
  it('strips 0x prefix and lowercases', () => {
    expect(normaliseHex('0xABcd')).toBe('abcd');
    expect(normaliseHex('0XABcd')).toBe('abcd');
    expect(normaliseHex('  ABcd  ')).toBe('abcd');
    expect(normaliseHex('ABcd')).toBe('abcd');
  });
});

describe('decodeHex', () => {
  it('decodes valid hex', () => {
    expect(decodeHex('dead')).toEqual(new Uint8Array([0xde, 0xad]));
    expect(decodeHex('0xdead', 2)).toEqual(new Uint8Array([0xde, 0xad]));
  });

  it('rejects odd length', () => {
    expect(() => decodeHex('abc')).toThrow(/odd length/);
  });

  it('rejects non-hex chars', () => {
    expect(() => decodeHex('gg')).toThrow(/non-hex/);
  });

  it('rejects wrong length when expected is set', () => {
    expect(() => decodeHex('dead', 32)).toThrow(/expected 32 bytes/);
  });
});

describe('loadProofBundle', () => {
  it('decodes a well-formed bundle', () => {
    const path = tmpFile(
      'bundle.json',
      JSON.stringify({
        proofHex: 'ab'.repeat(256),
        publicSignalsHex: ['cd'.repeat(32), 'ef'.repeat(32)],
      }),
    );
    const bundle = loadProofBundle(path);
    expect(bundle.proofBytes).toHaveLength(256);
    expect(bundle.publicSignals).toHaveLength(2);
    expect(bundle.publicSignals[0]).toHaveLength(32);
    expect(bundle.publicSignals[0]?.[0]).toBe(0xcd);
    expect(bundle.publicSignals[1]?.[0]).toBe(0xef);
  });

  it('accepts the 0x-prefixed hex form', () => {
    const path = tmpFile(
      'bundle.json',
      JSON.stringify({
        proofHex: `0x${'ab'.repeat(256)}`,
        publicSignalsHex: [`0x${'cd'.repeat(32)}`],
      }),
    );
    const bundle = loadProofBundle(path);
    expect(bundle.proofBytes[0]).toBe(0xab);
    expect(bundle.publicSignals[0]?.[0]).toBe(0xcd);
  });

  it('rejects an empty signals array', () => {
    const path = tmpFile(
      'bundle.json',
      JSON.stringify({ proofHex: 'ab'.repeat(256), publicSignalsHex: [] }),
    );
    expect(() => loadProofBundle(path)).toThrow(/must not be empty/);
  });

  it('rejects a malformed JSON file', () => {
    const path = tmpFile('bundle.json', '{not json');
    expect(() => loadProofBundle(path)).toThrow(/could not parse/);
  });

  it('rejects a proof of the wrong length', () => {
    const path = tmpFile(
      'bundle.json',
      JSON.stringify({ proofHex: 'ab', publicSignalsHex: ['cd'.repeat(32)] }),
    );
    expect(() => loadProofBundle(path)).toThrow(/expected 256 bytes/);
  });

  it('rejects a signal of the wrong length', () => {
    const path = tmpFile(
      'bundle.json',
      JSON.stringify({ proofHex: 'ab'.repeat(256), publicSignalsHex: ['cd'] }),
    );
    expect(() => loadProofBundle(path)).toThrow(/expected 32 bytes/);
  });
});

describe('loadKeypair', () => {
  it('round-trips a generated keypair', async () => {
    const { Keypair } = await import('@solana/web3.js');
    const kp = Keypair.generate();
    const path = tmpFile(
      'kp.json',
      JSON.stringify(Array.from(kp.secretKey)),
    );
    const loaded = loadKeypair(path);
    expect(loaded.publicKey.toBase58()).toBe(kp.publicKey.toBase58());
  });

  it('rejects an array of the wrong length', () => {
    const path = tmpFile('kp.json', JSON.stringify(Array(32).fill(0)));
    expect(() => loadKeypair(path)).toThrow(/64-element/);
  });

  it('rejects non-byte values', () => {
    const arr = Array(64).fill(0);
    arr[3] = 300;
    const path = tmpFile('kp.json', JSON.stringify(arr));
    expect(() => loadKeypair(path)).toThrow(/byte/);
  });
});
