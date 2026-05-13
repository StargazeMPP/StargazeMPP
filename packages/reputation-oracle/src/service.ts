import { aggregateAll, DEFAULT_HALF_LIFE_DAYS, DEFAULT_MIN_VOTES } from './aggregator.js';
import type { ScorePublisher } from './publisher.js';
import type { VoteSource } from './source.js';
import type { Vote } from './vote.js';

export interface ReputationOracleConfig {
  /** Where to read votes from. */
  source: VoteSource;
  /** Where to publish aggregated scores. */
  publisher: ScorePublisher;
  /**
   * Slot to start (and resume) reading from. Defaults to `0n` — i.e.,
   * full historical replay on the first tick. The oracle advances this
   * cursor in memory after each successful tick.
   */
  cursor?: bigint;
  /**
   * Minimum raw vote count before a provider's score is published.
   * Defaults to `DEFAULT_MIN_VOTES` (3).
   */
  minVotes?: number;
  /**
   * Optional time-decay half-life in days. When set, vote weight =
   * `0.5 ^ (ageDays / halfLifeDays)`. Defaults to `undefined` =
   * unweighted (every vote counts equally).
   */
  halfLifeDays?: number;
}

export interface TickResult {
  /** Providers for whom a score was published this tick. */
  published: number;
  /**
   * Votes that did not produce a published score — either because the
   * provider was below `minVotes`, or because no votes were loaded.
   */
  skipped: number;
  /**
   * Cursor value after the tick. Equal to `max(slot) + 1n` of the
   * loaded votes, or the previous cursor if no votes were loaded.
   */
  cursor: bigint;
}

/**
 * Reference oracle: orchestrates source → aggregator → publisher.
 *
 * Design notes:
 *   - `tick()` is the explicit unit of work; a deployment wrapper can
 *     loop on an interval, run on demand, or be triggered by a job
 *     queue. Keeping the loop out of the class makes it trivial to
 *     unit-test.
 *   - The cursor is in-memory only. Production deployments should
 *     persist it elsewhere (e.g., back into Postgres) and feed it back
 *     in via `cursor` on construction.
 *   - Publish errors propagate. A partial tick will leave the cursor at
 *     its pre-tick value; the operator re-runs after fixing the issue.
 */
export class ReputationOracle {
  private cursor: bigint;
  private readonly minVotes: number;
  private readonly halfLifeDays: number | undefined;

  constructor(private readonly cfg: ReputationOracleConfig) {
    this.cursor = cfg.cursor ?? 0n;
    this.minVotes = cfg.minVotes ?? DEFAULT_MIN_VOTES;
    this.halfLifeDays = cfg.halfLifeDays;
  }

  getCursor(): bigint {
    return this.cursor;
  }

  async tick(now: Date = new Date()): Promise<TickResult> {
    const votes = await this.cfg.source.loadSince(this.cursor);
    if (votes.length === 0) {
      return { published: 0, skipped: 0, cursor: this.cursor };
    }

    const scored = aggregateAll(votes, {
      minVotes: this.minVotes,
      halfLifeDays: this.halfLifeDays,
      now,
    });

    for (const s of scored) {
      await this.cfg.publisher.publish(s);
    }

    // Sum of votes that contributed to a published score.
    const publishedVoteCount = scored.reduce((acc, s) => acc + s.totalVotes, 0);
    const skipped = votes.length - publishedVoteCount;

    this.cursor = maxSlot(votes) + 1n;

    return {
      published: scored.length,
      skipped,
      cursor: this.cursor,
    };
  }
}

export { DEFAULT_HALF_LIFE_DAYS, DEFAULT_MIN_VOTES };

function maxSlot(votes: Vote[]): bigint {
  let m = 0n;
  for (const v of votes) {
    if (v.slot > m) m = v.slot;
  }
  return m;
}
