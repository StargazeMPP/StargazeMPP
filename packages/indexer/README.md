# `@stargazempp/indexer`

Real-time Solana indexer for `StargazeAnchor` events and x402 USDC receipts.

**Stack.** Rust · Tokio · Yellowstone gRPC · sqlx (planned) · structured JSON logging via `tracing`.

**Latency target.** Sub-50 milliseconds from on-chain finality to a row in the Postgres + TimescaleDB warehouse that the API gateway reads.

## Configure

The indexer reads from environment variables (and optionally `.env`):

| Variable | Purpose |
|---|---|
| `STARGAZE_NETWORK` | `solana-mainnet`, `solana-devnet`, or `localnet`. |
| `STARGAZE_ANCHOR_PROGRAM_ID` | Program ID published from [`@stargazempp/shared/solana`](../shared/src/solana/programs.ts). |
| `YELLOWSTONE_GRPC_URL` | Triton One / Helius Yellowstone endpoint. |
| `DATABASE_URL` | Postgres connection string (TimescaleDB extension enabled). |

## Run

```bash
cargo build --release
./target/release/stargaze-indexer
```

## Test

Unit tests (in-memory `VecSink`, no database required):

```bash
cargo test -p stargaze-indexer
```

The Postgres-backed integration test (`tests/postgres_sink.rs`) writes one
of every `DecodedEvent` variant, reads each row back, and verifies the
`ON CONFLICT (slot, signature) DO NOTHING` idempotency invariant. It is
gated by `#[ignore]` so CI without a database does not trip; invoke it
explicitly against an operator-provided Postgres:

```bash
DATABASE_URL=postgres://user:pass@localhost:5432/stargaze_test \
  cargo test -p stargaze-indexer --test postgres_sink -- --ignored
```

The test uses slots in the range `[100_000, 200_000)` and pre-cleans that
range from every projection table on entry, so concurrent runs against a
shared database don't collide with real ingestion data.

## Audit binaries

Three small read-only binaries cover the demo / on-call surfaces. All
read `DATABASE_URL` from env (or `.env`), all accept `--limit N`
(default 50, capped at 1000), all emit tab-separated rows that pipe
into `column -t`, `awk`, or `cut`. Cells that depend on a join row the
indexer never observed render `-`.

`stargaze-audit-vault-proofs` joins `vault_proof_verified` against the
latest `provider_registered` row per `provider_id`, ordered by
`created_at DESC`. Use it for the "show me which providers have posted
proofs recently" surface:

```bash
DATABASE_URL=postgres://user:pass@host/db \
  cargo run -p stargaze-indexer \
  --bin stargaze-audit-vault-proofs -- --limit 20
```

`stargaze-audit-vouchers` walks `voucher_settled` joined with the
latest `provider_registered` per `provider_id`. Each row carries the
monotonically-growing `cumulative_amount` settled-to-provider for its
`(session_id, provider_id)` pair, so the output doubles as a
per-session running-total tape:

```bash
DATABASE_URL=postgres://user:pass@host/db \
  cargo run -p stargaze-indexer \
  --bin stargaze-audit-vouchers -- --limit 20
```

`stargaze-audit-sessions` walks `session_settled` joined with the
originating `session_opened` row per `session_id`. Settlement is
one-shot per session, so each output row is the final state: original
deposit, agent wallet, and the three-way split. Eyeball invariant is
`deposit == total_to_providers + routing_fee + refund_to_agent`
(modulo any non-routing-fee skim a future ix might introduce):

```bash
DATABASE_URL=postgres://user:pass@host/db \
  cargo run -p stargaze-indexer \
  --bin stargaze-audit-sessions -- --limit 20
```
