# `@stargazempp/anchor-program`

`StargazeAnchor` — the Solana-side companion to the Tempo EVM contracts, and the home of the `$GAZE` token economy.

## `$GAZE` SPL

`$GAZE` is a standard Solana SPL launched on pump.fun. Fixed supply 1,000,000,000, 6 decimals, no transfer hooks. The mint address is wired into `stargaze_anchor` via `set_stake_mint` (authority-gated). All token logic — staking, slashing, future burns — lives in this program.

## Responsibilities

- **Provider mirror.** Solana-native providers register here; reputation scores are mirrored from Tempo via Chainlink CCIP.
- **x402 receipt store.** Records `X402Receipt` PDAs so the indexer can project Solana-rail payments into Postgres without re-fetching tx history.
- **Reputation votes.** Voters cast Solana-side votes which propagate to Tempo via CCIP for score aggregation.
- **`$GAZE` staking + burn ladder.** Per-provider stake escrow with cooldown, admin-gated slashing, plus the routing-fee burn ladder (50/50 burn-vs-staker-rewards) and the 1-`$GAZE` reputation-vote burn.

## Instructions

| Instruction | Purpose |
|---|---|
| `initialize` | One-time config PDA setup with authority key. |
| `register_provider` | Create a `Provider` PDA keyed by `provider_id`. |
| `cast_reputation_vote` | Emit a vote event that the CCIP relay forwards to Tempo. |
| `record_x402_receipt` | Persist an x402 USDC payment receipt for the indexer. |
| `ccip_mirror_score` | Authority-only — write the latest reputation score for a provider. |
| `dispatch_reputation_to_tempo` | Authority-only — emit a CCIP-formatted message mirroring a provider's score to the Tempo `StargazeCcipReceiver`. CPI to the Chainlink router is wired in M4; configure `CHAINLINK_CCIP_PROGRAM_ID` in `.env`. |
| `init_staking` | One-time staking config PDA setup. |
| `set_stake_mint` | Authority-only — pin the `$GAZE` SPL mint address. |
| `stake` | Lock `$GAZE` into a per-staker PDA (active stake counter). |
| `request_unstake` | Move a portion of stake into the cooling-down counter; starts the cooldown timer. |
| `claim_unstake` | After cooldown, withdraw the cooling-down balance back to the staker's token account. |
| `slash` | Authority-only — burn a portion of a staker's active stake by transferring to the incinerator address. |
| `process_routing_fee_burn` | Authority-only — split a routing-fee tranche 50/50 between an SPL `token::burn` and a transfer to the staker reward pool. Odd amounts favour stakers. |
| `reputation_vote_burn` | Voter-signed — burn 1 `$GAZE` (`VOTE_BURN_AMOUNT`) from the caller's ATA per reputation vote. |

### Staking parameters

| Constant | Value | Meaning |
|---|---|---|
| `MIN_STAKE` | `50_000_000` | 50 `$GAZE` @ 6 decimals — minimum to register as a provider. |
| `VERIFIED_STAKE` | `500_000_000` | 500 `$GAZE` — threshold for the Verified Provider tier. |
| `COOLDOWN` | `7 days` | Time between `request_unstake` and `claim_unstake`. |

A per-staker PDA tracks `active` and `cooling_down` amounts. Slashed tokens are transferred to the incinerator address (`1nc1nerator11111111111111111111111111111111`).

### Burn ladder

| Path | Instruction | Mechanism |
|---|---|---|
| Routing-fee burn | `process_routing_fee_burn(amount)` | 50/50 split — half is `token::burn`-ed (truly reducing SPL supply), half is transferred into the `staker_reward_pool_authority` PDA's ATA. Odd amounts route the extra base unit to stakers. Running totals on `StakingConfig.total_routing_fee_burned` and `StakingConfig.total_routing_fee_to_stakers`. |
| Reputation-vote burn | `reputation_vote_burn(provider_id)` | `token::burn` of exactly `VOTE_BURN_AMOUNT` (1 `$GAZE` @ 6 decimals) from the caller's ATA. Voter signs directly — no admin gate. |

The staker reward pool is **accumulation-only** — distribution mechanism (pull-based Merkle vs push-based proportional) is deferred and tracked separately.

### Future (M4)

- **Routing-fee admin gate → CCIP.** `process_routing_fee_burn` is admin-gated today. M4 swaps the gate for the CCIP fan-out so the Tempo `routingFeeBurn` message can drive the burn without a privileged signer; the 2% PathUSD routing fee bridges to Solana, swaps to `$GAZE`, and feeds the existing 50/50 split.

## Build & test

```bash
anchor build
cargo test -p stargaze_anchor_tests
```

IDL JSON is emitted to `target/idl/stargaze_anchor.json` and republished into [`@stargazempp/shared/solana`](../shared/src/solana/) for cross-package import.
