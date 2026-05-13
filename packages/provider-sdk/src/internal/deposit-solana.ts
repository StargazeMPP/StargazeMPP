import {
  Connection,
  type ParsedTransactionWithMeta,
  type ParsedInstruction,
  type PartiallyDecodedInstruction,
} from '@solana/web3.js';
import type { VerifiedDeposit } from '@stargazempp/shared';

const TOKEN_PROGRAM_IDS = new Set([
  'TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA', // SPL Token v1
  'TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb', // SPL Token-2022
]);

export interface SolanaDepositVerifierOptions {
  /** Solana RPC endpoint, e.g. `https://api.mainnet-beta.solana.com` or a Helius URL. */
  rpcUrl: string;
  /** USDC mint address (mainnet, devnet, or localnet — must match the tx). */
  usdcMint: string;
  /** Optional pre-built connection — primarily used in tests. */
  connection?: Connection;
}

interface QualifyingDeposit {
  agentWallet: string;
  amount: bigint;
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
 * Pure parser — given the inner + outer instructions of a Solana tx,
 * find the SPL Token transfer that lands in `expectedRecipient` with at
 * least `minAmount` of the configured USDC mint. Extracted from the RPC
 * client so unit tests can feed synthetic parsed instructions.
 *
 * Walks both top-level instructions and CPI-emitted inner instructions
 * (real x402 deposits typically wrap the transfer inside a higher-level
 * router CPI).
 */
export function findQualifyingSolanaDeposit(
  tx: Pick<ParsedTransactionWithMeta, 'transaction' | 'meta'>,
  usdcMint: string,
  expectedRecipient: string,
  minAmount: bigint,
): QualifyingDeposit | null {
  if (tx.meta?.err) return null;

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
    if (info.destination !== expectedRecipient) continue;

    // `transferChecked` carries the mint inline; plain `transfer` does not.
    // For `transfer` we trust the upstream filter (token account → mint is
    // a separate RPC). The session manager re-checks against the escrow's
    // token account mint at open time.
    if (info.mint && info.mint !== usdcMint) continue;

    const amountStr = info.tokenAmount?.amount ?? info.amount;
    if (!amountStr) continue;
    const amount = BigInt(amountStr);
    if (amount < minAmount) continue;

    const agentWallet = info.authority ?? info.source;
    if (!agentWallet) continue;

    return { agentWallet, amount };
  }
  return null;
}

/**
 * Verifies a Solana x402 USDC deposit by inspecting an on-chain tx.
 *
 * Flow:
 *   1. Fetch the parsed transaction at `signature` from the configured RPC.
 *   2. Walk top-level + inner instructions for an SPL Token `transfer` /
 *      `transferChecked` whose destination matches the escrow token account.
 *   3. Validate amount and (when carried inline) mint.
 *   4. Return the recovered payer authority + amount.
 */
export class SolanaDepositVerifier {
  private readonly connection: Connection;
  private readonly usdcMint: string;

  constructor(opts: SolanaDepositVerifierOptions) {
    this.connection = opts.connection ?? new Connection(opts.rpcUrl, 'confirmed');
    this.usdcMint = opts.usdcMint;
  }

  async verify(
    signature: string,
    expectedRecipient: string,
    minAmount: bigint,
  ): Promise<VerifiedDeposit> {
    const tx = await this.connection.getParsedTransaction(signature, {
      commitment: 'confirmed',
      maxSupportedTransactionVersion: 0,
    });
    if (!tx) {
      throw new Error(`SolanaDepositVerifier: transaction ${signature} not found`);
    }
    if (tx.meta?.err) {
      throw new Error(
        `SolanaDepositVerifier: transaction ${signature} failed (${JSON.stringify(tx.meta.err)})`,
      );
    }
    const match = findQualifyingSolanaDeposit(tx, this.usdcMint, expectedRecipient, minAmount);
    if (!match) {
      throw new Error(
        `SolanaDepositVerifier: no qualifying USDC transfer in ${signature} (to=${expectedRecipient}, minAmount=${minAmount})`,
      );
    }
    return {
      txHash: signature,
      agentWallet: match.agentWallet,
      amount: match.amount,
    };
  }
}
