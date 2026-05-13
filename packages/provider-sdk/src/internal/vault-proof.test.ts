import { createHash } from 'node:crypto';
import { describe, it, expect } from 'vitest';
import { PublicKey, SystemProgram } from '@solana/web3.js';
import {
  STARGAZE_ANCHOR_PROGRAM_ID,
  SUBMIT_VAULT_PROOF_DISCRIMINATOR,
  buildSubmitVaultProofInstruction,
  computeVaultProofSignalsHash,
  deriveVaultConfigPda,
  deriveVaultProofRecordPda,
} from './vault-proof.js';

function range32(start: number): Uint8Array {
  const out = new Uint8Array(32);
  for (let i = 0; i < 32; i++) out[i] = (start + i) & 0xff;
  return out;
}

function fill(len: number, byte: number): Uint8Array {
  const out = new Uint8Array(len);
  out.fill(byte);
  return out;
}

describe('computeVaultProofSignalsHash', () => {
  it('matches sha256 of the concatenated 32-byte signals', () => {
    const signals = [range32(0), range32(64)];
    const expected = new Uint8Array(
      createHash('sha256').update(signals[0]).update(signals[1]).digest(),
    );
    const got = computeVaultProofSignalsHash(signals);
    expect(Array.from(got)).toEqual(Array.from(expected));
  });

  it('returns 32 bytes for empty input (sha256 of nothing)', () => {
    const got = computeVaultProofSignalsHash([]);
    expect(got.length).toBe(32);
    // sha256("") = e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
    expect(Buffer.from(got).toString('hex')).toBe(
      'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855',
    );
  });

  it('rejects a non-32-byte signal', () => {
    expect(() => computeVaultProofSignalsHash([new Uint8Array(31)])).toThrow(
      /must be 32 bytes/,
    );
  });
});

describe('deriveVaultConfigPda', () => {
  it('uses seeds [b"vault", provider_id]', () => {
    const providerId = fill(32, 0x11);
    const got = deriveVaultConfigPda(providerId);
    const [expected] = PublicKey.findProgramAddressSync(
      [Buffer.from('vault'), Buffer.from(providerId)],
      STARGAZE_ANCHOR_PROGRAM_ID,
    );
    expect(got.toBase58()).toBe(expected.toBase58());
  });

  it('rejects a malformed provider id', () => {
    expect(() => deriveVaultConfigPda(new Uint8Array(31))).toThrow(/32 bytes/);
  });

  it('honours a custom stargazeProgramId', () => {
    const alt = new PublicKey('11111111111111111111111111111112');
    const providerId = fill(32, 0x22);
    const got = deriveVaultConfigPda(providerId, alt);
    const [expected] = PublicKey.findProgramAddressSync(
      [Buffer.from('vault'), Buffer.from(providerId)],
      alt,
    );
    expect(got.toBase58()).toBe(expected.toBase58());
  });
});

describe('deriveVaultProofRecordPda', () => {
  it('uses seeds [b"vault_proof", provider_id, signals_hash]', () => {
    const providerId = fill(32, 0x33);
    const signalsHash = fill(32, 0x44);
    const got = deriveVaultProofRecordPda(providerId, signalsHash);
    const [expected] = PublicKey.findProgramAddressSync(
      [Buffer.from('vault_proof'), Buffer.from(providerId), Buffer.from(signalsHash)],
      STARGAZE_ANCHOR_PROGRAM_ID,
    );
    expect(got.toBase58()).toBe(expected.toBase58());
  });

  it('rejects malformed inputs', () => {
    expect(() => deriveVaultProofRecordPda(new Uint8Array(31), fill(32, 0))).toThrow(
      /providerId/,
    );
    expect(() => deriveVaultProofRecordPda(fill(32, 0), new Uint8Array(31))).toThrow(
      /signalsHash/,
    );
  });
});

describe('buildSubmitVaultProofInstruction', () => {
  const submitter = new PublicKey('SysvarRent111111111111111111111111111111111');
  const verifierProgramId = new PublicKey(
    'CTC7ehb1sYj7A5EsAd3E6viYdo5bxydzSpccDENbkUmP', // aggregate_sum
  );

  it('produces a 5-account ix with the expected key order + writability', () => {
    const providerId = fill(32, 0x55);
    const proofBytes = new Uint8Array(256);
    for (let i = 0; i < 256; i++) proofBytes[i] = i;
    const publicSignals = [range32(100)];

    const { instruction, signalsHash, proofRecordPda, vaultConfigPda } =
      buildSubmitVaultProofInstruction({
        submitter,
        verifierProgramId,
        providerId,
        proofBytes,
        publicSignals,
      });

    expect(instruction.programId.toBase58()).toBe(STARGAZE_ANCHOR_PROGRAM_ID.toBase58());

    expect(instruction.keys.length).toBe(5);
    expect(instruction.keys[0].pubkey.toBase58()).toBe(submitter.toBase58());
    expect(instruction.keys[0].isSigner).toBe(true);
    expect(instruction.keys[0].isWritable).toBe(true);

    expect(instruction.keys[1].pubkey.toBase58()).toBe(vaultConfigPda.toBase58());
    expect(instruction.keys[1].isSigner).toBe(false);
    expect(instruction.keys[1].isWritable).toBe(false);

    expect(instruction.keys[2].pubkey.toBase58()).toBe(verifierProgramId.toBase58());
    expect(instruction.keys[2].isSigner).toBe(false);
    expect(instruction.keys[2].isWritable).toBe(false);

    expect(instruction.keys[3].pubkey.toBase58()).toBe(proofRecordPda.toBase58());
    expect(instruction.keys[3].isSigner).toBe(false);
    expect(instruction.keys[3].isWritable).toBe(true);

    expect(instruction.keys[4].pubkey.toBase58()).toBe(SystemProgram.programId.toBase58());

    expect(signalsHash.length).toBe(32);
  });

  it('encodes data as [discriminator | provider_id | signals_hash | proof | vec-len | signals]', () => {
    const providerId = fill(32, 0x66);
    const proofBytes = new Uint8Array(256);
    for (let i = 0; i < 256; i++) proofBytes[i] = 0xa0 + (i % 16);
    const publicSignals = [range32(10), range32(20), range32(30)];

    const { instruction, signalsHash } = buildSubmitVaultProofInstruction({
      submitter,
      verifierProgramId,
      providerId,
      proofBytes,
      publicSignals,
    });

    const data = new Uint8Array(instruction.data);
    expect(data.length).toBe(8 + 32 + 32 + 256 + 4 + 3 * 32);

    expect(Array.from(data.slice(0, 8))).toEqual(
      Array.from(SUBMIT_VAULT_PROOF_DISCRIMINATOR),
    );
    expect(Array.from(data.slice(8, 40))).toEqual(Array.from(providerId));
    expect(Array.from(data.slice(40, 72))).toEqual(Array.from(signalsHash));
    expect(Array.from(data.slice(72, 328))).toEqual(Array.from(proofBytes));

    // Vec length is u32 little-endian.
    expect(data[328]).toBe(3);
    expect(data[329]).toBe(0);
    expect(data[330]).toBe(0);
    expect(data[331]).toBe(0);

    // Signals in declaration order.
    expect(Array.from(data.slice(332, 364))).toEqual(Array.from(publicSignals[0]));
    expect(Array.from(data.slice(364, 396))).toEqual(Array.from(publicSignals[1]));
    expect(Array.from(data.slice(396, 428))).toEqual(Array.from(publicSignals[2]));
  });

  it('rejects malformed shapes', () => {
    const providerId = fill(32, 0x77);
    expect(() =>
      buildSubmitVaultProofInstruction({
        submitter,
        verifierProgramId,
        providerId,
        proofBytes: new Uint8Array(255), // wrong
        publicSignals: [],
      }),
    ).toThrow(/proofBytes/);

    expect(() =>
      buildSubmitVaultProofInstruction({
        submitter,
        verifierProgramId,
        providerId: new Uint8Array(31), // wrong
        proofBytes: new Uint8Array(256),
        publicSignals: [],
      }),
    ).toThrow(/providerId/);

    expect(() =>
      buildSubmitVaultProofInstruction({
        submitter,
        verifierProgramId,
        providerId,
        proofBytes: new Uint8Array(256),
        publicSignals: [new Uint8Array(31)], // wrong
      }),
    ).toThrow(/publicSignals/);
  });
});
