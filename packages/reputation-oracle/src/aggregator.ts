import type { AggregatedScore, Vote } from './vote.js';

/**
 * Default minimum number of votes required before a provider gets a
 * published score. Chosen to avoid scoring brand-new providers off a
 * single vote (the on-chain default is 500 / neutral midpoint until the
 * oracle posts an update).
 */
export const DEFAULT_MIN_VOTES = 3;

/**
 * Default half-life in days for the optional time-decay weighting. Each
 * vote's weight is `0.5 ^ (ageDays / HALF_LIFE_DAYS)` when decay is on.
 */
export const DEFAULT_HALF_LIFE_DAYS = 30;

/** Max reputation score that `StargazeRegistry.setReputationScore` accepts. */
export const MAX_REPUTATION = 1000;

export interface AggregateOptions {
  /**
   * Minimum number of votes (raw count, not weighted) before a provider's
   * score is emitted. Defaults to `DEFAULT_MIN_VOTES`.
   */
  minVotes?: number;
  /**
   * If set, apply exponential time-decay weighting with this half-life
   * (in days). Older votes count less. Defaults to `undefined` =
   * unweighted (every vote counts equally).
   */
  halfLifeDays?: number;
  /**
   * "Now" reference point for decay calculations. Inject for tests so
   * results stay deterministic. Defaults to `new Date()`.
   */
  now?: Date;
}

/**
 * Composite reputation formula (v1).
 *
 * ```
 *   score = round(MAX_REPUTATION * accurateWeight / totalWeight)
 *   score ∈ [0, MAX_REPUTATION]   (clamped)
 * ```
 *
 * Where `accurateWeight` is the sum of per-vote weights for votes with
 * `accurate === true`, and `totalWeight` is the sum across all votes.
 *
 * Unweighted (default): every vote contributes weight 1, so the score
 * reduces to `round(1000 * accurateVotes / totalVotes)`.
 *
 * Optional time decay: when `halfLifeDays` is provided, each vote's
 * weight is `0.5 ^ (ageDays / halfLifeDays)`, so a vote one half-life old
 * counts half as much as a fresh one. Aggregation remains pure — the
 * caller passes `now` (defaulting to `new Date()`) so results stay
 * reproducible in tests.
 *
 * Returns `null` if fewer than `minVotes` raw votes are present for the
 * provider (regardless of weighting). This avoids publishing noisy
 * scores for brand-new providers; the on-chain initial value of 500
 * (neutral midpoint) is used instead until the threshold is hit.
 *
 * Pre-conditions:
 *   - All `votes` must share the same `providerId`. Callers that have
 *     mixed input should use {@link aggregateAll} instead.
 */
export function aggregateVotes(
  votes: Vote[],
  opts: AggregateOptions = {},
): AggregatedScore | null {
  const minVotes = opts.minVotes ?? DEFAULT_MIN_VOTES;
  if (votes.length < minVotes) return null;

  const first = votes[0];
  if (!first) return null;
  const providerId = first.providerId;

  let totalWeight = 0;
  let accurateWeight = 0;
  let accurateCount = 0;
  const now = opts.now ?? new Date();

  for (const v of votes) {
    const weight = computeWeight(v, now, opts.halfLifeDays);
    totalWeight += weight;
    if (v.accurate) {
      accurateWeight += weight;
      accurateCount += 1;
    }
  }

  if (totalWeight <= 0) return null;

  const raw = (MAX_REPUTATION * accurateWeight) / totalWeight;
  const score = clamp(Math.round(raw), 0, MAX_REPUTATION);

  return {
    providerId,
    score,
    totalVotes: votes.length,
    accurateVotes: accurateCount,
  };
}

/**
 * Bucket a heterogeneous batch of votes by `providerId`, run
 * {@link aggregateVotes} on each bucket, and drop unscored providers.
 *
 * Returns one entry per provider whose vote count meets `minVotes`.
 * Output order is deterministic: sorted by `providerId` ascending so
 * downstream consumers (publishers, snapshots) get stable diffs.
 */
export function aggregateAll(
  votes: Vote[],
  opts: AggregateOptions = {},
): AggregatedScore[] {
  const buckets = new Map<`0x${string}`, Vote[]>();
  for (const v of votes) {
    const bucket = buckets.get(v.providerId);
    if (bucket) bucket.push(v);
    else buckets.set(v.providerId, [v]);
  }

  const out: AggregatedScore[] = [];
  for (const bucket of buckets.values()) {
    const score = aggregateVotes(bucket, opts);
    if (score) out.push(score);
  }
  out.sort((a, b) => (a.providerId < b.providerId ? -1 : a.providerId > b.providerId ? 1 : 0));
  return out;
}

function computeWeight(vote: Vote, now: Date, halfLifeDays: number | undefined): number {
  if (halfLifeDays === undefined) return 1;
  if (halfLifeDays <= 0) return 1;
  const ageMs = now.getTime() - vote.votedAt.getTime();
  if (ageMs <= 0) return 1;
  const ageDays = ageMs / (1000 * 60 * 60 * 24);
  return Math.pow(0.5, ageDays / halfLifeDays);
}

function clamp(value: number, lo: number, hi: number): number {
  if (value < lo) return lo;
  if (value > hi) return hi;
  return value;
}
