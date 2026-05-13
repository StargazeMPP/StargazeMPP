import { describe, it, expect } from 'vitest';
import { aggregateAll, aggregateVotes, DEFAULT_MIN_VOTES } from './aggregator.js';
import { MemoryPublisher } from './publisher.js';
import { ReputationOracle } from './service.js';
import type { VoteSource } from './source.js';
import type { Vote } from './vote.js';

const PROVIDER_A = `0x${'aa'.repeat(32)}` as `0x${string}`;
const PROVIDER_B = `0x${'bb'.repeat(32)}` as `0x${string}`;

function makeVote(overrides: Partial<Vote> & { accurate: boolean }): Vote {
  return {
    providerId: PROVIDER_A,
    voter: 'voter-fixture',
    accurate: overrides.accurate,
    slot: 1n,
    votedAt: new Date('2026-05-13T00:00:00Z'),
    ...overrides,
  };
}

function repeat(accurate: boolean, count: number, base: Partial<Vote> = {}): Vote[] {
  return Array.from({ length: count }, (_, i) =>
    makeVote({ accurate, slot: BigInt(i + 1), ...base }),
  );
}

describe('aggregateVotes', () => {
  it('1. returns null when below default minVotes', () => {
    const votes = [makeVote({ accurate: true })];
    expect(aggregateVotes(votes)).toBeNull();
    expect(votes.length).toBeLessThan(DEFAULT_MIN_VOTES);
  });

  it('2. five accurate votes → score 1000', () => {
    const result = aggregateVotes(repeat(true, 5));
    expect(result).not.toBeNull();
    expect(result!.score).toBe(1000);
    expect(result!.totalVotes).toBe(5);
    expect(result!.accurateVotes).toBe(5);
  });

  it('3. five inaccurate votes → score 0', () => {
    const result = aggregateVotes(repeat(false, 5));
    expect(result).not.toBeNull();
    expect(result!.score).toBe(0);
    expect(result!.accurateVotes).toBe(0);
  });

  it('4. three accurate + one inaccurate → score 750', () => {
    const votes = [...repeat(true, 3), makeVote({ accurate: false, slot: 4n })];
    const result = aggregateVotes(votes);
    expect(result).not.toBeNull();
    expect(result!.score).toBe(750);
    expect(result!.totalVotes).toBe(4);
    expect(result!.accurateVotes).toBe(3);
  });

  it('7. cap: 100 accurate, 0 inaccurate → exactly 1000 (no overshoot)', () => {
    const result = aggregateVotes(repeat(true, 100));
    expect(result).not.toBeNull();
    expect(result!.score).toBe(1000);
    expect(result!.score).toBeLessThanOrEqual(1000);
  });
});

describe('aggregateAll', () => {
  it('5. two providers, only the above-threshold one is returned', () => {
    const votes: Vote[] = [
      ...repeat(true, 3, { providerId: PROVIDER_A }),
      // PROVIDER_B has only 2 votes — below the default threshold of 3.
      makeVote({ providerId: PROVIDER_B, accurate: true, slot: 100n }),
      makeVote({ providerId: PROVIDER_B, accurate: false, slot: 101n }),
    ];
    const result = aggregateAll(votes);
    expect(result).toHaveLength(1);
    expect(result[0]!.providerId).toBe(PROVIDER_A);
    expect(result[0]!.score).toBe(1000);
  });

  it('8. empty input → empty result', () => {
    expect(aggregateAll([])).toEqual([]);
  });

  it('6. half-life weighting halves the influence of a one-half-life-old vote', () => {
    const now = new Date('2026-05-13T00:00:00Z');
    const halfLifeAgo = new Date(now.getTime() - 30 * 24 * 60 * 60 * 1000);

    // Baseline: 3 fresh accurate votes → 1000 (every vote weight ≈ 1).
    const freshOnly = repeat(true, 3, { votedAt: now });
    const baseline = aggregateAll(freshOnly, { halfLifeDays: 30, now });
    expect(baseline).toHaveLength(1);
    expect(baseline[0]!.score).toBe(1000);

    // Mixed: 3 fresh inaccurate votes + 1 *old* accurate vote.
    // The old vote has weight 0.5, so:
    //   accurateWeight = 0.5
    //   totalWeight   = 3 + 0.5 = 3.5
    //   score = round(1000 * 0.5 / 3.5) = round(142.857) = 143.
    // Compare to the unweighted equivalent: 1 accurate / 4 total =
    //   round(1000 * 0.25) = 250. The decay should pull the score
    //   strictly *below* the unweighted value.
    const mixed: Vote[] = [
      ...repeat(false, 3, { votedAt: now, providerId: PROVIDER_B }),
      makeVote({ accurate: true, slot: 99n, providerId: PROVIDER_B, votedAt: halfLifeAgo }),
    ];

    const decayed = aggregateAll(mixed, { halfLifeDays: 30, now });
    expect(decayed).toHaveLength(1);
    expect(decayed[0]!.score).toBe(143);

    const undecayed = aggregateAll(mixed, { now });
    expect(undecayed[0]!.score).toBe(250);
    expect(decayed[0]!.score).toBeLessThan(undecayed[0]!.score);
  });
});

describe('ReputationOracle.tick', () => {
  class MemoryVoteSource implements VoteSource {
    constructor(public readonly votes: Vote[]) {}
    async loadSince(slot: bigint): Promise<Vote[]> {
      return this.votes
        .filter((v) => v.slot >= slot)
        .sort((a, b) => (a.slot < b.slot ? -1 : a.slot > b.slot ? 1 : 0));
    }
  }

  it('9. reads from MemoryVoteSource, publishes to MemoryPublisher, advances cursor', async () => {
    const votes: Vote[] = [
      ...repeat(true, 3, { providerId: PROVIDER_A }),
      // Provider B below threshold — should not be published.
      makeVote({ providerId: PROVIDER_B, accurate: true, slot: 50n }),
    ];
    const source = new MemoryVoteSource(votes);
    const publisher = new MemoryPublisher();
    const oracle = new ReputationOracle({ source, publisher });

    expect(oracle.getCursor()).toBe(0n);

    const result = await oracle.tick();
    expect(result.published).toBe(1);
    expect(publisher.published).toHaveLength(1);
    expect(publisher.published[0]!.providerId).toBe(PROVIDER_A);
    expect(publisher.published[0]!.score).toBe(1000);
    // Max slot in fixture is 50 (the singleton Provider-B vote).
    expect(result.cursor).toBe(51n);
    expect(oracle.getCursor()).toBe(51n);

    // Second tick: no new votes past the cursor — cursor steady,
    // nothing published.
    const second = await oracle.tick();
    expect(second.published).toBe(0);
    expect(second.skipped).toBe(0);
    expect(oracle.getCursor()).toBe(51n);
    expect(publisher.published).toHaveLength(1);
  });
});
