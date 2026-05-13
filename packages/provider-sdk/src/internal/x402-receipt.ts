import type {
  ParsedTransactionWithMeta,
  ParsedInstruction,
  PartiallyDecodedInstruction,
} from '@solana/web3.js';
import type { X402Receipt } from '@stargazempp/shared';

const TOKEN_PROGRAM_IDS = new Set([
  'TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA', // SPL Token v1
  'TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb', // SPL Token-2022
]);

/** Options for {@link parseX402Receipt}. */
export interface X402ReceiptParserOptions {
  /** Canonical USDC mint (mainnet, devnet, or localnet — must match the tx). */
  usdcMint: string;
  /** Expected recipient (escrow PDA or provider ATA — must match the transfer destination). */
  expectedRecipient: string;
  /** Minimum acceptable amount. Use `1n` to mean "any positive amount." */
  minAmount?: bigint;
}

interface ParsedTokenInstructionInfo {
  destination?: string;
  source?: string;
  authority?: string;
  mint?: string;
  amount?: string;
  tokenAmount?: { amount: string; decimals: number };
}

function isParsed(ix: ParsedInstruction | PartiallyDecodedInstruction): ix is ParsedInstruction {
  return 'parsed' in ix;
}

/**
 * Parse a Solana x402 USDC transfer into a typed receipt.
 *
 * Symmetric to the on-chain `X402ReceiptRecorded` Anchor event decoder in
 * the Rust indexer, but works directly off the raw `ParsedTransactionWithMeta`
 * so providers can verify a payment *before* the Anchor program has recorded
 * it (e.g. at session-open time, in parallel with
 * {@link findQualifyingSolanaDeposit}).
 *
 * Walks both top-level instructions and CPI-emitted inner instructions —
 * real x402 deposits typically nest the SPL Token transfer inside a higher-
 * level router CPI.
 *
 * Returns `null` if any of the following hold:
 *   - `tx.meta.err` is set (the tx failed),
 *   - no SPL Token `transfer` / `transferChecked` of `usdcMint` lands in
 *     `expectedRecipient`,
 *   - the matching amount is below `minAmount` (defaults to `1n`),
 *   - `tx.blockTime` is `null`/`undefined` (the receipt schema requires
 *     `paidAt`),
 *   - the matching instruction lacks an `authority` (no recoverable payer),
 *   - the tx has no signatures.
 *
 * **Contract:** when multiple qualifying transfers exist in a single tx, the
 * first one encountered (outer instructions first, then inner) is returned.
 */
export function parseX402Receipt(
  tx: Pick<ParsedTransactionWithMeta, 'transaction' | 'meta' | 'blockTime'>,
  opts: X402ReceiptParserOptions,
): X402Receipt | null {
  if (tx.meta?.err) return null;
  if (tx.blockTime === null || tx.blockTime === undefined) return null;

  const signature = tx.transaction.signatures[0];
  if (!signature) return null;

  const minAmount = opts.minAmount ?? 1n;

  const outer = tx.transaction.message.instructions as Array<
    ParsedInstruction | PartiallyDecodedInstruction
  >;
  const inner =
    tx.meta?.innerInstructions?.flatMap(
      (ii) => ii.instructions as Array<ParsedInstruction | PartiallyDecodedInstruction>,
    ) ?? [];

  for (const ix of [...outer, ...inner]) {
    if (!isParsed(ix)) continue;
    if (!TOKEN_PROGRAM_IDS.has(ix.programId.toBase58())) continue;
    if (ix.parsed.type !== 'transferChecked' && ix.parsed.type !== 'transfer') continue;

    const info = ix.parsed.info as ParsedTokenInstructionInfo;
    if (info.destination !== opts.expectedRecipient) continue;

    // `transferChecked` carries the mint inline; plain `transfer` does not.
    // For `transfer` the caller is asserting the destination is a USDC token
    // account out-of-band (the session manager re-checks the escrow's token
    // account mint at open time).
    if (info.mint && info.mint !== opts.usdcMint) continue;

    const amountStr = info.tokenAmount?.amount ?? info.amount;
    if (!amountStr) continue;
    const amount = BigInt(amountStr);
    if (amount < minAmount) continue;

    const payer = info.authority;
    if (!payer) continue;

    return {
      signature,
      payer,
      recipient: opts.expectedRecipient,
      mint: opts.usdcMint,
      amount,
      paidAt: tx.blockTime,
    };
  }
  return null;
}
