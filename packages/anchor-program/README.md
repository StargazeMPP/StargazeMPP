# `@stargazempp/anchor-program`

`StargazeAnchor` — the Solana-side companion to the Tempo EVM contracts.

## Responsibilities

- **Provider mirror.** Solana-native providers register here without needing to bridge `$GAZE` upfront; reputation scores are mirrored from Tempo via Chainlink CCIP.
- **x402 receipt store.** Records `X402Receipt` PDAs so the indexer can project Solana-rail payments into Postgres without re-fetching tx history.
- **Reputation votes.** Voters cast Solana-side votes which propagate to Tempo via CCIP for `$GAZE` burn accounting.

## Instructions

| Instruction | Purpose |
|---|---|
| `initialize` | One-time config PDA setup with authority key. |
| `register_provider` | Create a `Provider` PDA keyed by `provider_id`. |
| `cast_reputation_vote` | Emit a vote event that the CCIP relay forwards to Tempo. |
| `record_x402_receipt` | Persist an x402 USDC payment receipt for the indexer. |
| `ccip_mirror_score` | Authority-only — write the latest reputation score for a provider. |
| `dispatch_reputation_to_tempo` | Authority-only — emit a CCIP-formatted message mirroring a provider's score to the Tempo `StargazeCcipReceiver`. CPI to the Chainlink router is wired in M4; configure `CHAINLINK_CCIP_PROGRAM_ID` in `.env`. |

## Build & test

```bash
anchor build
cargo test -p stargaze_anchor_tests
```

IDL JSON is emitted to `target/idl/stargaze_anchor.json` and republished into [`@stargazempp/shared/solana`](../shared/src/solana/) for cross-package import.
