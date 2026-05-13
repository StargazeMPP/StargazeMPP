import type { SignedVoucher, VerifiedVoucher } from './voucher.js';

export interface DepositProof {
  txHash: string;
}

export interface VerifiedDeposit {
  txHash: string;
  agentWallet: string;
  /** Smallest unit (USDC base units, 6 decimals). */
  amount: bigint;
}

/**
 * The cryptographic primitive that turns signed bytes into a verified payer
 * and amount. Implemented by this team; consumed by the backend's session
 * manager and by `@stargazempp/provider-sdk`.
 *
 * Implementations MUST NOT enforce session-level rules (monotonicity,
 * spending limit, expiry); the session manager owns those. This interface
 * is purely about recovering identity and amount from a signature.
 */
export interface MppVerifier {
  /**
   * Verify an on-chain Solana deposit. Hits the configured Solana RPC. Used
   * once per session at `session.open` — not on the hot path.
   *
   * Throws if the tx doesn't exist, is unconfirmed, transfers the wrong
   * asset, sends less than `minAmount`, or pays a different recipient.
   */
  verifyDeposit(
    proof: DepositProof,
    expectedRecipient: string,
    minAmount: bigint,
  ): Promise<VerifiedDeposit>;

  /**
   * Recover the agent wallet from a Solana voucher by verifying the Ed25519
   * signature over the 133-byte voucher message. Pure crypto — no RPC, no
   * I/O. Sub-10ms budget on the hot path.
   *
   * Async only by convention; resolves synchronously in practice.
   *
   * Throws on signature recovery failure or domain mismatch.
   */
  verifyVoucher(voucher: SignedVoucher): Promise<VerifiedVoucher>;
}
