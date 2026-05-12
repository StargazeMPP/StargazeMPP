import type {
  DepositProof,
  MppVerifier,
  SignedVoucher,
  VerifiedDeposit,
  VerifiedVoucher,
} from '@stargazempp/shared';
import { recoverVoucherSigner } from './voucher.js';

export interface StargazeMppVerifierOptions {
  /** Tempo EVM RPC endpoint. Required for `verifyDeposit` on the `tempo` rail. */
  tempoRpcUrl?: string;
  /** Solana RPC endpoint. Required for `verifyDeposit` on the `solana` rail. */
  solanaRpcUrl?: string;
  /** Expected Tempo chain ID (e.g. mainnet vs testnet). */
  tempoChainId?: number;
  /** Address of the `StargazeEscrow` contract that holds session deposits on Tempo. */
  tempoEscrowAddress?: `0x${string}`;
}

/**
 * Reference implementation of `MppVerifier`. Voucher signature recovery
 * is fully implemented (via viem); deposit verification is a stub pending
 * Tempo testnet RPC details and the Solana indexer's projection of x402
 * receipts.
 *
 * Once `verifyDeposit` is wired:
 *   - **tempo rail** → fetch tx receipt from `tempoRpcUrl`, decode the
 *     PathUSD `Transfer(from, to, value)` event(s), assert `to ==
 *     tempoEscrowAddress`, assert `value >= minAmount`, return `from`
 *     as `agentWallet`.
 *   - **solana rail** → read the `X402Receipt` PDA produced by
 *     `StargazeAnchor.record_x402_receipt`, assert `recipient ==
 *     expectedRecipient`, assert `amount >= minAmount`, return `payer`
 *     as `agentWallet`.
 */
export class StargazeMppVerifier implements MppVerifier {
  constructor(private readonly opts: StargazeMppVerifierOptions = {}) {}

  async verifyDeposit(
    _proof: DepositProof,
    _expectedRecipient: string,
    _minAmount: bigint,
  ): Promise<VerifiedDeposit> {
    throw new Error(
      'StargazeMppVerifier.verifyDeposit: not yet implemented — Tempo testnet RPC pending (BLOCKERS.md)',
    );
  }

  verifyVoucher(voucher: SignedVoucher): Promise<VerifiedVoucher> {
    return recoverVoucherSigner(voucher);
  }
}
