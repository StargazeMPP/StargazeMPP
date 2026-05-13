/**
 * Domain types for the reputation oracle.
 *
 * A `Vote` is the off-chain projection of an Anchor `ReputationVoted` event
 * (see `packages/anchor-program/programs/stargaze_anchor/src/lib.rs`). The
 * indexer (`packages/indexer/`) decodes the on-chain event and persists one
 * row per cast vote in the `reputation_voted` Postgres table. The oracle
 * reads those rows, aggregates per-provider, and writes a composite
 * reputation score back on-chain via `StargazeRegistry.setReputationScore`.
 */

/**
 * A single accuracy vote cast by a Solana agent against a provider.
 *
 * Field shapes mirror what the indexer stores (`provider_id` and `voter`
 * are raw 32-byte values in Postgres BYTEA columns; the oracle re-encodes
 * them as 0x-prefixed hex for `providerId` and base58 for `voter`).
 */
export interface Vote {
  /** 32-byte providerId, encoded as 0x-prefixed hex. */
  providerId: `0x${string}`;
  /** Base58-encoded Solana voter pubkey. */
  voter: string;
  /** `true` if the voter judged the provider's response accurate. */
  accurate: boolean;
  /** Solana slot the underlying event was emitted at. */
  slot: bigint;
  /** Transaction signature (optional — defaults to empty in the indexer). */
  signature?: string;
  /**
   * Wall-clock time the indexer ingested the vote (`created_at` column).
   * Used only by the optional time-decay weighting; aggregation is otherwise
   * order- and time-independent.
   */
  votedAt: Date;
}

/**
 * Per-provider aggregate ready to publish on-chain.
 *
 * `score` is the value that will be passed to
 * `StargazeRegistry.setReputationScore(bytes32, uint256)`. The contract
 * reverts with `ScoreOutOfRange` if `score > 1000`, so callers must keep
 * the value clamped to `[0, 1000]`.
 */
export interface AggregatedScore {
  providerId: `0x${string}`;
  /** Composite score in `[0, 1000]`. */
  score: number;
  /** Total votes counted (after any decay-weight filtering). */
  totalVotes: number;
  /** Subset of `totalVotes` that voted accurate. */
  accurateVotes: number;
}
