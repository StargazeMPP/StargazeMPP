import { performance } from 'node:perf_hooks';
import { describe, it, expect } from 'vitest';
import { privateKeyToAccount, generatePrivateKey } from 'viem/accounts';
import type { SignedVoucher, VoucherMessage } from '@stargazempp/shared';
import { VOUCHER_TYPES, VOUCHER_PRIMARY_TYPE, buildVoucherDomain } from '@stargazempp/shared';
import { recoverVoucherSigner } from './voucher.js';
import { StargazeMppVerifier } from './verifier.js';

const ESCROW_ADDRESS = '0x1234567890123456789012345678901234567890' as const;
const PROVIDER_ADDRESS = '0xabcdefabcdefabcdefabcdefabcdefabcdefabcd' as const;
const SESSION_ID = `0x${'ca'.repeat(32)}` as `0x${string}`;
const CHAIN_ID = 31337;

async function signVoucher(privateKey: `0x${string}`, message: VoucherMessage): Promise<SignedVoucher> {
  const account = privateKeyToAccount(privateKey);
  const domain = buildVoucherDomain(CHAIN_ID, ESCROW_ADDRESS);
  const signature = await account.signTypedData({
    domain,
    types: VOUCHER_TYPES,
    primaryType: VOUCHER_PRIMARY_TYPE,
    message,
  });
  return { domain, message, signature };
}

describe('recoverVoucherSigner', () => {
  it('recovers the address that signed a well-formed voucher', async () => {
    const privateKey = generatePrivateKey();
    const account = privateKeyToAccount(privateKey);
    const message: VoucherMessage = {
      sessionId: SESSION_ID,
      agentWallet: account.address,
      provider: PROVIDER_ADDRESS,
      cumulativeAmount: 25_000_000n,
      nonce: 1n,
      expiry: BigInt(Math.floor(Date.now() / 1000) + 3600),
    };

    const voucher = await signVoucher(privateKey, message);
    const verified = await recoverVoucherSigner(voucher);

    expect(verified.agentWallet.toLowerCase()).toBe(account.address.toLowerCase());
    expect(verified.cumulativeAmount).toBe(25_000_000n);
    expect(verified.provider).toBe(PROVIDER_ADDRESS);
    expect(verified.sessionId).toBe(SESSION_ID);
    expect(verified.nonce).toBe(1n);
  });

  it('recovers a *different* address when the cumulativeAmount is tampered post-signing', async () => {
    const privateKey = generatePrivateKey();
    const account = privateKeyToAccount(privateKey);
    const message: VoucherMessage = {
      sessionId: SESSION_ID,
      agentWallet: account.address,
      provider: PROVIDER_ADDRESS,
      cumulativeAmount: 25_000_000n,
      nonce: 1n,
      expiry: BigInt(Math.floor(Date.now() / 1000) + 3600),
    };

    const voucher = await signVoucher(privateKey, message);
    const tampered: SignedVoucher = {
      ...voucher,
      message: { ...voucher.message, cumulativeAmount: 99_999_999n },
    };
    const verified = await recoverVoucherSigner(tampered);

    expect(verified.agentWallet.toLowerCase()).not.toBe(account.address.toLowerCase());
  });

  it('runs comfortably under the 10 ms hot-path budget', async () => {
    const privateKey = generatePrivateKey();
    const account = privateKeyToAccount(privateKey);
    const message: VoucherMessage = {
      sessionId: SESSION_ID,
      agentWallet: account.address,
      provider: PROVIDER_ADDRESS,
      cumulativeAmount: 1n,
      nonce: 1n,
      expiry: BigInt(Math.floor(Date.now() / 1000) + 3600),
    };
    const voucher = await signVoucher(privateKey, message);

    // Warm-up — viem lazy-initialises some internals on first call.
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

describe('StargazeMppVerifier', () => {
  it('delegates voucher verification to recoverVoucherSigner', async () => {
    const verifier = new StargazeMppVerifier();
    const privateKey = generatePrivateKey();
    const account = privateKeyToAccount(privateKey);
    const message: VoucherMessage = {
      sessionId: SESSION_ID,
      agentWallet: account.address,
      provider: PROVIDER_ADDRESS,
      cumulativeAmount: 1n,
      nonce: 1n,
      expiry: BigInt(Math.floor(Date.now() / 1000) + 3600),
    };
    const voucher = await signVoucher(privateKey, message);

    const verified = await verifier.verifyVoucher(voucher);
    expect(verified.agentWallet.toLowerCase()).toBe(account.address.toLowerCase());
  });

  it('refuses tempo verifyDeposit without the required configuration', async () => {
    const verifier = new StargazeMppVerifier();
    await expect(
      verifier.verifyDeposit({ txHash: '0xfeed', rail: 'tempo' }, '0xbeef', 100n),
    ).rejects.toThrow(/Tempo deposit verification requires/i);
  });

  it('refuses solana verifyDeposit until the rail is wired', async () => {
    const verifier = new StargazeMppVerifier();
    await expect(
      verifier.verifyDeposit({ txHash: 'sig', rail: 'solana' }, 'recipient', 100n),
    ).rejects.toThrow(/Solana deposit verification not yet implemented/i);
  });
});
