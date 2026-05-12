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
