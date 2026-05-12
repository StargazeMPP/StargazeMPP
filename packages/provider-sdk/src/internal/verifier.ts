import type {
  DepositProof,
  MppVerifier,
  SignedVoucher,
  VerifiedDeposit,
  VerifiedVoucher,
} from '@stargazempp/shared';
import type { Address, Chain, Hex, PublicClient } from 'viem';
import { recoverVoucherSigner } from './voucher.js';
import { TempoDepositVerifier } from './deposit-tempo.js';

export interface StargazeMppVerifierOptions {
  /** Tempo EVM RPC endpoint. Required for `verifyDeposit` on the `tempo` rail. */
  tempoRpcUrl?: string;
  /** Optional pre-built viem `PublicClient` for tempo, mainly used in tests. */
  tempoClient?: PublicClient;
  /** Optional viem chain definition for tempo, used when constructing the default client. */
  tempoChain?: Chain;
  /** Address of the PathUSD ERC-20 on the target Tempo network. */
  tempoPathUsdAddress?: Address;
  /** Solana RPC endpoint. Required for `verifyDeposit` on the `solana` rail. */
  solanaRpcUrl?: string;
}

/**
 * Reference implementation of `MppVerifier`.
 *
 * - Voucher recovery is implemented via viem's EIP-712 `recoverTypedDataAddress`.
 * - Tempo deposit verification is implemented via `TempoDepositVerifier`,
 *   which parses a tx receipt's ERC-20 Transfer logs.
 * - Solana deposit verification reads `X402Receipt` PDAs from
 *   `StargazeAnchor`; wired separately in `deposit-solana.ts`.
 */
export class StargazeMppVerifier implements MppVerifier {
  private readonly tempoVerifier?: TempoDepositVerifier;

  constructor(private readonly opts: StargazeMppVerifierOptions = {}) {
    if (opts.tempoPathUsdAddress && (opts.tempoRpcUrl || opts.tempoClient)) {
      this.tempoVerifier = new TempoDepositVerifier({
        rpcUrl: opts.tempoRpcUrl ?? '',
        chain: opts.tempoChain,
        pathUsdAddress: opts.tempoPathUsdAddress,
        client: opts.tempoClient,
      });
    }
  }

  async verifyDeposit(
    proof: DepositProof,
    expectedRecipient: string,
    minAmount: bigint,
  ): Promise<VerifiedDeposit> {
    if (proof.rail === 'tempo') {
      if (!this.tempoVerifier) {
        throw new Error(
          'StargazeMppVerifier: Tempo deposit verification requires `tempoRpcUrl` (or `tempoClient`) plus `tempoPathUsdAddress`.',
        );
      }
      return this.tempoVerifier.verify(
        proof.txHash as Hex,
        expectedRecipient as Address,
        minAmount,
      );
    }
    if (proof.rail === 'solana') {
      throw new Error(
        'StargazeMppVerifier: Solana deposit verification not yet implemented — coming next.',
      );
    }
    throw new Error(`StargazeMppVerifier: unknown rail '${(proof as { rail: string }).rail}'`);
  }

  verifyVoucher(voucher: SignedVoucher): Promise<VerifiedVoucher> {
    return recoverVoucherSigner(voucher);
  }
}
