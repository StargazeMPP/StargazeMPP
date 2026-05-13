import { describe, it, expect } from 'vitest';
import { PublicKey, type ParsedTransactionWithMeta } from '@solana/web3.js';
import { parseX402Receipt } from './x402-receipt.js';

const USDC_MAINNET = 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v';
const ESCROW_TOKEN_ACCT = 'EscrwTokAcc11111111111111111111111111111111';
const AGENT_AUTHORITY = 'Ag3ntAuth111111111111111111111111111111111';
const SIG = '5w4FaakA6q9LhUDhT4tEdNNcfBP2vKzKvVoG8nzkPnGoYZmTpJ1ZWcVgZAv1z9pNuRoZTRJrTfQAuLNg4cpzAo7t';
const PAID_AT = 1_715_000_000;
const TOKEN_PROGRAM = new PublicKey('TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA');
const OTHER_PROGRAM = new PublicKey('11111111111111111111111111111111');

function buildTx(
  outer: Array<Record<string, unknown>>,
  inner: Array<Record<string, unknown>> = [],
  overrides: {
    err?: unknown;
    blockTime?: number | null;
    signatures?: string[];
  } = {},
): Pick<ParsedTransactionWithMeta, 'transaction' | 'meta' | 'blockTime'> {
  return {
    transaction: {
      message: { instructions: outer },
      signatures: overrides.signatures ?? [SIG],
    } as unknown as ParsedTransactionWithMeta['transaction'],
    meta: {
      err: overrides.err ?? null,
      fee: 0,
      preBalances: [],
      postBalances: [],
      innerInstructions: inner.length
        ? [{ index: 0, instructions: inner } as never]
        : [],
    } as unknown as ParsedTransactionWithMeta['meta'],
    blockTime: overrides.blockTime === undefined ? PAID_AT : overrides.blockTime,
  };
}

function transferChecked(opts: {
  destination: string;
  source?: string;
  authority?: string;
  mint?: string;
  amount: string;
  decimals?: number;
  programId?: PublicKey;
}) {
  return {
    programId: opts.programId ?? TOKEN_PROGRAM,
    parsed: {
      type: 'transferChecked',
      info: {
        destination: opts.destination,
        source: opts.source ?? 'SourceTokAcc11111111111111111111111111111',
        authority: opts.authority ?? AGENT_AUTHORITY,
        mint: opts.mint ?? USDC_MAINNET,
        tokenAmount: { amount: opts.amount, decimals: opts.decimals ?? 6 },
      },
    },
  };
}

function transferPlain(opts: {
  destination: string;
  source?: string;
  authority?: string;
  amount: string;
  programId?: PublicKey;
}) {
  return {
    programId: opts.programId ?? TOKEN_PROGRAM,
    parsed: {
      type: 'transfer',
      info: {
        destination: opts.destination,
        source: opts.source ?? 'SourceTokAcc11111111111111111111111111111',
        authority: opts.authority ?? AGENT_AUTHORITY,
        amount: opts.amount,
      },
    },
  };
}

describe('parseX402Receipt', () => {
  it('parses a valid transferChecked into a full X402Receipt', () => {
    const tx = buildTx([
      transferChecked({ destination: ESCROW_TOKEN_ACCT, amount: '100000000' }),
    ]);
    const receipt = parseX402Receipt(tx, {
      usdcMint: USDC_MAINNET,
      expectedRecipient: ESCROW_TOKEN_ACCT,
      minAmount: 50_000_000n,
    });
    expect(receipt).not.toBeNull();
    expect(receipt).toEqual({
      signature: SIG,
      payer: AGENT_AUTHORITY,
      recipient: ESCROW_TOKEN_ACCT,
      mint: USDC_MAINNET,
      amount: 100_000_000n,
      paidAt: PAID_AT,
    });
  });

  it('parses a plain transfer (no inline mint) using opts.usdcMint', () => {
    const tx = buildTx([
      transferPlain({ destination: ESCROW_TOKEN_ACCT, amount: '42000000' }),
    ]);
    const receipt = parseX402Receipt(tx, {
      usdcMint: USDC_MAINNET,
      expectedRecipient: ESCROW_TOKEN_ACCT,
    });
    expect(receipt?.mint).toBe(USDC_MAINNET);
    expect(receipt?.amount).toBe(42_000_000n);
    expect(receipt?.payer).toBe(AGENT_AUTHORITY);
    expect(receipt?.recipient).toBe(ESCROW_TOKEN_ACCT);
  });

  it('returns null when the tx failed (meta.err set)', () => {
    const tx = buildTx(
      [transferChecked({ destination: ESCROW_TOKEN_ACCT, amount: '100000000' })],
      [],
      { err: { InstructionError: [0, 'Custom'] } },
    );
    expect(
      parseX402Receipt(tx, {
        usdcMint: USDC_MAINNET,
        expectedRecipient: ESCROW_TOKEN_ACCT,
      }),
    ).toBeNull();
  });

  it('returns null when transfer destination is a different recipient', () => {
    const tx = buildTx([
      transferChecked({ destination: 'NotEscrow', amount: '100000000' }),
    ]);
    expect(
      parseX402Receipt(tx, {
        usdcMint: USDC_MAINNET,
        expectedRecipient: ESCROW_TOKEN_ACCT,
      }),
    ).toBeNull();
  });

  it('returns null for transferChecked of the wrong mint', () => {
    const tx = buildTx([
      transferChecked({
        destination: ESCROW_TOKEN_ACCT,
        amount: '100000000',
        mint: 'NotUSDC1111111111111111111111111111111111',
      }),
    ]);
    expect(
      parseX402Receipt(tx, {
        usdcMint: USDC_MAINNET,
        expectedRecipient: ESCROW_TOKEN_ACCT,
      }),
    ).toBeNull();
  });

  it('returns null when amount is below minAmount', () => {
    const tx = buildTx([
      transferChecked({ destination: ESCROW_TOKEN_ACCT, amount: '1000' }),
    ]);
    expect(
      parseX402Receipt(tx, {
        usdcMint: USDC_MAINNET,
        expectedRecipient: ESCROW_TOKEN_ACCT,
        minAmount: 5000n,
      }),
    ).toBeNull();
  });

  it('returns null when blockTime is missing (null)', () => {
    const tx = buildTx(
      [transferChecked({ destination: ESCROW_TOKEN_ACCT, amount: '100000000' })],
      [],
      { blockTime: null },
    );
    expect(
      parseX402Receipt(tx, {
        usdcMint: USDC_MAINNET,
        expectedRecipient: ESCROW_TOKEN_ACCT,
      }),
    ).toBeNull();
  });

  it('matches a transfer nested inside an inner-instruction CPI', () => {
    const tx = buildTx(
      [{ programId: OTHER_PROGRAM, parsed: { type: 'noop', info: {} } }],
      [transferChecked({ destination: ESCROW_TOKEN_ACCT, amount: '7500000' })],
    );
    const receipt = parseX402Receipt(tx, {
      usdcMint: USDC_MAINNET,
      expectedRecipient: ESCROW_TOKEN_ACCT,
    });
    expect(receipt?.amount).toBe(7_500_000n);
    expect(receipt?.payer).toBe(AGENT_AUTHORITY);
    expect(receipt?.signature).toBe(SIG);
  });

  it('returns the first qualifying transfer when two are present (contract)', () => {
    const tx = buildTx([
      transferChecked({
        destination: ESCROW_TOKEN_ACCT,
        amount: '1000000',
        authority: AGENT_AUTHORITY,
      }),
      transferChecked({
        destination: ESCROW_TOKEN_ACCT,
        amount: '9999999',
        authority: 'OtherAuth111111111111111111111111111111111',
      }),
    ]);
    const receipt = parseX402Receipt(tx, {
      usdcMint: USDC_MAINNET,
      expectedRecipient: ESCROW_TOKEN_ACCT,
    });
    expect(receipt?.amount).toBe(1_000_000n);
    expect(receipt?.payer).toBe(AGENT_AUTHORITY);
  });
});
