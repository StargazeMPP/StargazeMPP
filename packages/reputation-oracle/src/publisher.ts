import type { Address, WalletClient } from 'viem';
import { STARGAZE_REGISTRY_ABI } from '@stargazempp/shared';
import type { AggregatedScore } from './vote.js';

/**
 * Anything that can deliver a computed `AggregatedScore` to its sink.
 * Production uses {@link EvmPublisher}; tests use {@link MemoryPublisher}.
 */
export interface ScorePublisher {
  publish(score: AggregatedScore): Promise<void>;
}

export interface EvmPublisherConfig {
  /**
   * A `WalletClient` whose default account holds `ORACLE_ROLE` on the
   * `StargazeRegistry` at `registryAddress`. Admin must
   * `grantRole(ORACLE_ROLE, walletClient.account.address)` before the
   * oracle starts publishing, or every `setReputationScore` call will
   * revert with `AccessControlUnauthorizedAccount`.
   */
  walletClient: WalletClient;
  /** Deployed `StargazeRegistry` contract address. */
  registryAddress: Address;
}

/**
 * Publishes scores on-chain via `StargazeRegistry.setReputationScore`
 * (`onlyRole(ORACLE_ROLE)`). The score is passed as a `bigint` because
 * the on-chain parameter is `uint256` (clamped at 1000); the aggregator
 * guarantees `score ∈ [0, 1000]` so any uint256 overflow concerns are
 * moot.
 */
export class EvmPublisher implements ScorePublisher {
  constructor(private readonly cfg: EvmPublisherConfig) {}

  async publish(s: AggregatedScore): Promise<void> {
    const { walletClient, registryAddress } = this.cfg;
    const account = walletClient.account;
    if (!account) {
      throw new Error('EvmPublisher: walletClient has no default account');
    }
    const chain = walletClient.chain ?? null;
    await walletClient.writeContract({
      account,
      chain,
      address: registryAddress,
      abi: STARGAZE_REGISTRY_ABI,
      functionName: 'setReputationScore',
      args: [s.providerId, BigInt(s.score)],
    });
  }
}

/**
 * In-memory publisher used by tests. Records every published score for
 * later assertion. No side effects beyond the in-process array.
 */
export class MemoryPublisher implements ScorePublisher {
  public readonly published: AggregatedScore[] = [];

  async publish(s: AggregatedScore): Promise<void> {
    this.published.push(s);
  }
}
