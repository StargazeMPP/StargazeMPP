import type {
  DepositProof,
  MppVerifier,
  SignedVoucher,
  VerifiedDeposit,
  VerifiedVoucher,
} from '@stargazempp/shared';
import type { Connection } from '@solana/web3.js';
import { recoverVoucherSigner } from './voucher.js';
import { SolanaDepositVerifier } from './deposit-solana.js';

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
