/**
 * Solana-side x402 USDC receipt — the Coinbase / Solana Foundation flow that
 * runs in parallel to Tempo PathUSD vouchers. Parsed by the indexer and by
 * `@stargazempp/provider-sdk`'s `parseX402Receipt` for the session manager.
 */
export interface X402Receipt {
  /** Solana transaction signature (base58). */
  signature: string;
  /** Solana pubkey (base58) of the agent paying. */
  payer: string;
  /** Solana pubkey of the StargazeEscrow Solana-side mirror. */
  recipient: string;
  /** USDC mint address (base58); should equal the canonical mainnet/devnet USDC mint. */
  mint: string;
  /** Smallest unit of USDC (6 decimals). */
  amount: bigint;
  /** Unix seconds. */
  paidAt: number;
}
