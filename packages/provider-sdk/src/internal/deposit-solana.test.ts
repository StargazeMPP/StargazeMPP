import { describe, it, expect } from 'vitest';
import { PublicKey, type ParsedTransactionWithMeta } from '@solana/web3.js';
import { findQualifyingSolanaDeposit } from './deposit-solana.js';

const USDC_MAINNET = 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v';
const ESCROW_TOKEN_ACCT = 'EscrwTokAcc11111111111111111111111111111111';
const AGENT_AUTHORITY = 'Ag3ntAuth111111111111111111111111111111111';
const TOKEN_PROGRAM = new PublicKey('TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA');
const OTHER_PROGRAM = new PublicKey('11111111111111111111111111111111');

function buildTx(
  outer: Array<Record<string, unknown>>,
  inner: Array<Record<string, unknown>> = [],
  err: unknown = null,
): Pick<ParsedTransactionWithMeta, 'transaction' | 'meta'> {
  return {
    transaction: {
      // Structural cast: only the fields the parser reads are required.
      message: { instructions: outer },
      signatures: [],
    } as unknown as ParsedTransactionWithMeta['transaction'],
    meta: {
      err,
      fee: 0,
      preBalances: [],
      postBalances: [],
      innerInstructions: inner.length
        ? [{ index: 0, instructions: inner } as never]
        : [],
    } as unknown as ParsedTransactionWithMeta['meta'],
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

describe('findQualifyingSolanaDeposit', () => {
  it('matches a top-level transferChecked into the escrow', () => {
    const tx = buildTx([
      transferChecked({ destination: ESCROW_TOKEN_ACCT, amount: '100000000' }),
    ]);
    const result = findQualifyingSolanaDeposit(tx, USDC_MAINNET, ESCROW_TOKEN_ACCT, 50_000_000n);
    expect(result?.amount).toBe(100_000_000n);
    expect(result?.agentWallet).toBe(AGENT_AUTHORITY);
  });

  it('matches a CPI-emitted inner transfer', () => {
    const tx = buildTx(
      [{ programId: OTHER_PROGRAM, parsed: { type: 'noop', info: {} } }],
      [transferChecked({ destination: ESCROW_TOKEN_ACCT, amount: '7500000' })],
    );
    const result = findQualifyingSolanaDeposit(tx, USDC_MAINNET, ESCROW_TOKEN_ACCT, 1n);
    expect(result?.amount).toBe(7_500_000n);
  });

  it('rejects a transfer below minAmount', () => {
    const tx = buildTx([
      transferChecked({ destination: ESCROW_TOKEN_ACCT, amount: '1000' }),
    ]);
    expect(
      findQualifyingSolanaDeposit(tx, USDC_MAINNET, ESCROW_TOKEN_ACCT, 5000n),
    ).toBeNull();
  });

  it('rejects a transfer to a different recipient', () => {
    const tx = buildTx([
      transferChecked({ destination: 'NotEscrow', amount: '100000000' }),
    ]);
    expect(
      findQualifyingSolanaDeposit(tx, USDC_MAINNET, ESCROW_TOKEN_ACCT, 1n),
    ).toBeNull();
  });

  it('rejects transferChecked of the wrong mint', () => {
    const tx = buildTx([
      transferChecked({
        destination: ESCROW_TOKEN_ACCT,
        amount: '100000000',
        mint: 'NotUSDC1111111111111111111111111111111111',
      }),
    ]);
    expect(
      findQualifyingSolanaDeposit(tx, USDC_MAINNET, ESCROW_TOKEN_ACCT, 1n),
    ).toBeNull();
  });

  it('ignores instructions from non-Token programs', () => {
    const tx = buildTx([
      {
        programId: OTHER_PROGRAM,
        parsed: {
          type: 'transferChecked',
          info: {
            destination: ESCROW_TOKEN_ACCT,
            tokenAmount: { amount: '999999999', decimals: 6 },
            mint: USDC_MAINNET,
            authority: AGENT_AUTHORITY,
          },
        },
      },
    ]);
    expect(
      findQualifyingSolanaDeposit(tx, USDC_MAINNET, ESCROW_TOKEN_ACCT, 1n),
    ).toBeNull();
  });

  it('returns null when the tx has an error', () => {
    const tx = buildTx(
      [transferChecked({ destination: ESCROW_TOKEN_ACCT, amount: '100000000' })],
      [],
      { InstructionError: [0, 'Custom'] },
    );
    expect(
      findQualifyingSolanaDeposit(tx, USDC_MAINNET, ESCROW_TOKEN_ACCT, 1n),
    ).toBeNull();
  });
});
