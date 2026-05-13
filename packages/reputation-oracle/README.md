# `@stargazempp/reputation-oracle`

Thin reference implementation of the StargazeMPP reputation oracle.
Reads `ReputationVoted` events from the indexer's Postgres projection,
aggregates them into per-provider composite scores in `[0, 1000]`, and
publishes the result on-chain via
`StargazeRegistry.setReputationScore` (`onlyRole(ORACLE_ROLE)`).

## Scoring math (v1)

```
score = round(MAX_REPUTATION * accurateWeight / totalWeight)
      ∈ [0, 1000]   (clamped)
```

- **Unweighted (default)** — every vote contributes weight `1`. The
  formula reduces to `round(1000 * accurateVotes / totalVotes)`.
- **Optional time decay** — when `HALF_LIFE_DAYS` is set, each vote's
  weight is `0.5 ^ (ageDays / HALF_LIFE_DAYS)`. A vote one half-life
  old counts half as much as a fresh one.
- **Min votes** — providers with fewer than `MIN_VOTES` raw votes are
  not scored. The contract keeps its initial `500` neutral midpoint
  until the threshold is reached. Default = `3`.

The aggregator (`src/aggregator.ts`) is a pure function; `now` is
injectable so test results stay deterministic.

## Environment variables

| Var | Purpose | Default |
| --- | --- | --- |
| `DATABASE_URL` | Postgres connection string with the indexer's `reputation_voted` table. | — |
| `RPC_URL` | Tempo / EVM JSON-RPC endpoint. | — |
| `ORACLE_PRIVATE_KEY` | EOA holding `ORACLE_ROLE` on `StargazeRegistry`. | — |
| `REGISTRY_ADDRESS` | Deployed `StargazeRegistry` address. | — |
| `MIN_VOTES` | Minimum raw votes before a score is published. | `3` |
| `HALF_LIFE_DAYS` | If set, enables time-decay weighting. | unset |

## Pre-flight

The admin must `grantRole(ORACLE_ROLE, publisher_eoa)` on the deployed
`StargazeRegistry` before the oracle starts. Without it, every
`setReputationScore` call reverts with
`AccessControlUnauthorizedAccount`.

## Known limitations

- **Unweighted votes.** No stake or reputation-of-the-voter weighting.
  An adversary with many low-stake agents can move scores at a 1:1
  cost per vote (each vote already burns 1 `$GAZE` via
  `BurnController.burnForReputationVoteFrom`, which is the only
  Sybil deterrent).
- **Pull-based.** The oracle reads from Postgres on `tick()`. It is
  not realtime — latency is `(indexer lag) + (tick interval)`.
- **In-memory cursor.** Restarts replay from `cursor = 0n` unless the
  deployment wrapper persists the cursor elsewhere and feeds it back
  via the `ReputationOracleConfig`.
- **No per-provider rate limiting.** Every provider with new votes
  above the threshold receives one on-chain write per tick.
  Production should debounce repeated identical scores.
- **No Bayesian smoothing, no stake weighting, no EWMA.** These are
  explicit v1.1 territory.
