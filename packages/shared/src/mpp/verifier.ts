import type { SignedVoucher } from './voucher.js';
import type { PaymentRail } from './session.js';

export interface DepositProof {
  txHash: string;
  rail: PaymentRail;
}

export interface VerifiedDeposit {
  txHash: string;
  rail: PaymentRail;
  agentWallet: string;
  /** Smallest unit (wei on Tempo PathUSD, lamports on Solana USDC). */
  amount: bigint;
}

export interface VerifiedVoucher {
  sessionId: string;
  agentWallet: string;
  provider: string;
  cumulativeAmount: bigint;
  nonce: bigint;
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
   * Verify an on-chain deposit. Hits the appropriate RPC. Used once per
   * session at `session.open` — not on the hot path.
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
   * Recover the agent wallet from a signed voucher via EIP-712 `ecrecover`.
   * Pure crypto — no RPC, no I/O. Sub-10ms budget on the hot path.
   *
   * Throws on signature recovery failure or domain / type mismatch.
   */
  verifyVoucher(voucher: SignedVoucher): VerifiedVoucher;
}
