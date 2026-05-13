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

## Audit binary

`stargaze-audit-vault-proofs` is a small read-only binary that joins
`vault_proof_verified` against the latest `provider_registered` row per
`provider_id`, ordered by `created_at DESC`. Use it for the "show me
which providers have posted proofs recently" surface (ToB demo, on-call
spot checks):

```bash
DATABASE_URL=postgres://user:pass@host/db \
  cargo run -p stargaze-indexer \
  --bin stargaze-audit-vault-proofs -- --limit 20
```

Output is tab-separated so it pipes into `column -t`, `awk`, or `cut`.
`--limit` defaults to 50 and is capped at 1000. Providers that have
never been seen by the indexer render `-` in the owner/category cells.
