# `packages/indexer` — Rust + Yellowstone gRPC

**Owner:** this team.

Sub-50ms lag indexer for all Solana-side MPP events: `StargazeAnchor` program logs, USDC voucher settlements, x402 receipts. Writes denormalized projections that `packages/backend` reads via Postgres + TimescaleDB hypertables.
