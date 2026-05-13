import { performance } from 'node:perf_hooks';
import { describe, it, expect } from 'vitest';
import nacl from 'tweetnacl';
import type { SignedVoucher } from '@stargazempp/shared';
import { VOUCHER_DOMAIN_TAG, buildVoucherMessage } from '@stargazempp/shared';
import { recoverVoucherSigner } from './voucher.js';

function makeSignedVoucher(opts?: {
  cumulativeAmount?: bigint;
  nonce?: bigint;
}): { voucher: SignedVoucher; keyPair: nacl.SignKeyPair } {
  const keyPair = nacl.sign.keyPair();
  const sessionId = new Uint8Array(32).fill(0xca);
  const providerId = new Uint8Array(32).fill(0xbe);
  const message = buildVoucherMessage({
    sessionId,
    agentWallet: keyPair.publicKey,
    providerId,
    cumulativeAmount: opts?.cumulativeAmount ?? 25_000_000n,
    nonce: opts?.nonce ?? 1n,
  });
  const signature = nacl.sign.detached(message, keyPair.secretKey);
  return {
    voucher: { message, signature, agentWallet: keyPair.publicKey },
    keyPair,
  };
}

describe('recoverVoucherSigner', () => {
  it('recovers a well-formed signed voucher', async () => {
    const { voucher, keyPair } = makeSignedVoucher({
      cumulativeAmount: 25_000_000n,
      nonce: 7n,
    });

    const verified = await recoverVoucherSigner(voucher);

    expect(Array.from(verified.agentWallet)).toEqual(Array.from(keyPair.publicKey));
    expect(verified.cumulativeAmount).toBe(25_000_000n);
    expect(verified.nonce).toBe(7n);
    expect(verified.sessionId.length).toBe(32);
    expect(verified.sessionId.every((b) => b === 0xca)).toBe(true);
    expect(verified.providerId.length).toBe(32);
    expect(verified.providerId.every((b) => b === 0xbe)).toBe(true);
  });

  it('rejects a voucher whose signature was tampered post-signing', async () => {
    const { voucher } = makeSignedVoucher();
    const tampered: SignedVoucher = {
      ...voucher,
      signature: new Uint8Array(voucher.signature),
    };
    tampered.signature[0] ^= 0x01;

    await expect(recoverVoucherSigner(tampered)).rejects.toThrow(
      /Ed25519 signature verification failed/i,
    );
  });

  it('rejects a voucher whose message was tampered post-signing', async () => {
    const { voucher } = makeSignedVoucher();
    const tampered: SignedVoucher = {
      ...voucher,
      message: new Uint8Array(voucher.message),
    };
    // Flip a byte inside the cumulative_amount field (offset 117..125).
    tampered.message[120] ^= 0x01;

    await expect(recoverVoucherSigner(tampered)).rejects.toThrow(
      /Ed25519 signature verification failed/i,
    );
  });

  it('rejects a voucher whose domain prefix is wrong', async () => {
    const { voucher } = makeSignedVoucher();
    const tampered: SignedVoucher = {
      ...voucher,
      message: new Uint8Array(voucher.message),
    };
    // Overwrite the first 21 bytes (domain).
    for (let i = 0; i < VOUCHER_DOMAIN_TAG.length; i++) {
      tampered.message[i] = 0x00;
    }

    await expect(recoverVoucherSigner(tampered)).rejects.toThrow(
      /voucher domain prefix mismatch/i,
    );
  });

  it('runs comfortably under the 10 ms hot-path budget', async () => {
    const { voucher } = makeSignedVoucher({ cumulativeAmount: 1n, nonce: 1n });

    // Warm-up.
    await recoverVoucherSigner(voucher);

    const ITERATIONS = 100;
    const start = performance.now();
    for (let i = 0; i < ITERATIONS; i++) {
      await recoverVoucherSigner(voucher);
    }
    const elapsedMs = performance.now() - start;
    const avgMs = elapsedMs / ITERATIONS;

    expect(avgMs).toBeLessThan(10);
  });
});
