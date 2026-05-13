// Public surface of the StargazeMPP reference reputation oracle.
//
// Composition: PostgresVoteSource → aggregateAll → EvmPublisher, wired
// together by `ReputationOracle.tick()`. See `README.md` for the math
// and operational pre-flight.

export type { Vote, AggregatedScore } from './vote.js';

export {
  aggregateVotes,
  aggregateAll,
  DEFAULT_MIN_VOTES,
  DEFAULT_HALF_LIFE_DAYS,
  MAX_REPUTATION,
  type AggregateOptions,
} from './aggregator.js';

export { type VoteSource, PostgresVoteSource } from './source.js';

export {
  type ScorePublisher,
  type EvmPublisherConfig,
  EvmPublisher,
  MemoryPublisher,
} from './publisher.js';

export {
  ReputationOracle,
  type ReputationOracleConfig,
  type TickResult,
} from './service.js';
