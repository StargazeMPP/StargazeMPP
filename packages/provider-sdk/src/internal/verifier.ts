import type { SignedVoucher, VerifiedVoucher } from '@stargazempp/shared';
import type { Connection } from '@solana/web3.js';
import { recoverVoucherSigner } from './voucher.js';
import { SolanaDepositVerifier } from './deposit-solana.js';

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
 * The cryptographic primitive that turns signed bytes into a verified
 * payer and amount. Implemented by `StargazeMppVerifier` below; any
 * future session-manager implementation (e.g. the external-dev
 * backend) can target the same contract.
 *
 * Implementations MUST NOT enforce session-level rules (monotonicity,
 * spending limit, expiry); that's the session manager's job. This
 * contract is purely about recovering identity and amount from a
 * signature.
 */
export interface MppVerifier {
  /**
   * Verify an on-chain Solana deposit. Hits the configured Solana RPC.
   * Used once per session at `session.open` — not on the hot path.
   *
   * Throws if the tx doesn't exist, is unconfirmed, transfers the
   * wrong asset, sends less than `minAmount`, or pays a different
   * recipient.
   */
  verifyDeposit(
    proof: DepositProof,
    expectedRecipient: string,
    minAmount: bigint,
  ): Promise<VerifiedDeposit>;

  /**
   * Recover the agent wallet from a Solana voucher by verifying the
   * Ed25519 signature over the 133-byte voucher message. Pure crypto —
   * no RPC, no I/O. Sub-10ms budget on the hot path.
   *
   * Async only by convention; resolves synchronously in practice.
   *
   * Throws on signature recovery failure or domain mismatch.
   */
  verifyVoucher(voucher: SignedVoucher): Promise<VerifiedVoucher>;
}

export interface StargazeMppVerifierOptions {
  /** Solana RPC endpoint. Required for `verifyDeposit`. */
  solanaRpcUrl?: string;
  /** Optional pre-built `Connection`, mainly used in tests. */
  solanaConnection?: Connection;
  /** Canonical USDC mint on the target Solana network. */
  solanaUsdcMint?: string;
}

/**
 * Reference implementation of `MppVerifier`.
 *
 * - Voucher recovery validates the Ed25519 signature over the 133-byte
 *   StargazeMPP voucher message via `recoverVoucherSigner`.
 * - Deposit verification reads SPL Token transfers from a parsed Solana tx
 *   via `SolanaDepositVerifier`.
 */
export class StargazeMppVerifier implements MppVerifier {
  private readonly solanaVerifier?: SolanaDepositVerifier;

  constructor(private readonly opts: StargazeMppVerifierOptions = {}) {
    if (opts.solanaUsdcMint && (opts.solanaRpcUrl || opts.solanaConnection)) {
      this.solanaVerifier = new SolanaDepositVerifier({
        rpcUrl: opts.solanaRpcUrl ?? '',
        usdcMint: opts.solanaUsdcMint,
        connection: opts.solanaConnection,
      });
    }
  }

  async verifyDeposit(
    proof: DepositProof,
    expectedRecipient: string,
    minAmount: bigint,
  ): Promise<VerifiedDeposit> {
    if (!this.solanaVerifier) {
      throw new Error(
        'StargazeMppVerifier: Solana deposit verification requires `solanaRpcUrl` (or `solanaConnection`) plus `solanaUsdcMint`.',
      );
    }
    return this.solanaVerifier.verify(proof.txHash, expectedRecipient, minAmount);
  }

  verifyVoucher(voucher: SignedVoucher): Promise<VerifiedVoucher> {
    return recoverVoucherSigner(voucher);
  }
}
