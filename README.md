# StargazeMPP

> Agentic intelligence marketplace built natively on the Machine Payments Protocol.

Agents discover, access, and pay for on-chain intelligence in a single HTTP request cycle. No accounts. No API keys. No KYC. Pay per query. Earn per response. Privacy opt-in via Groth16. Dual-rail Tempo PathUSD + Solana USDC (x402).

## Monorepo

| Package | Owner | Responsibility |
|---|---|---|
| `packages/backend` | external dev | Express 5 + tRPC 11 API, StargazeIndex Service, MPP Session Manager (orchestration), Reputation Oracle (elizaOS v2 + Claude API), Drizzle / Postgres + TimescaleDB, Redis. |
| `packages/frontend` | external dev | Next.js 15 marketplace UI (deep-space / star-violet). |
| `packages/provider-sdk` | shared | `@stargazempp/provider-sdk` — ergonomics by external dev, crypto primitives by this team. |
| `packages/contracts-evm` | this team | `GAZEToken`, `BurnController`, `StargazeEscrow`, `StargazeRegistry`, `PrivacyVaultRegistry` on Tempo EVM. |
| `packages/anchor-program` | this team | `StargazeAnchor` Solana program + CCIP bridge to Tempo. |
| `packages/indexer` | this team | Rust + Yellowstone gRPC, sub-50ms lag. |
| `packages/vault-circuits` | this team | Groth16 circuits + on-chain verifiers for StargazeVault privacy tiers. |
| `packages/shared` | shared | Types, ABIs, IDL, EIP-712 schema, Drizzle schema source of truth. |

See `SCOPE.md` for the work-split rationale and `EXTERNAL_DEV_BRIEF.md` for the external dev's task overview.

## Source docs

- `docs/overview.pdf` — product / business overview (10 pages).
- `docs/backend.pdf` — backend infrastructure spec (10 pages).

## Stack (from `docs/backend.pdf` §1)

Express 5 · tRPC 11 · elizaOS v2 · PostgreSQL + TimescaleDB · Redis (Upstash) · Tempo SDK · Solana MPP SDK (`solana-mpp`) · snarkjs + Groth16 · Drizzle ORM · Bun / Node 20+ · Claude API.

Contracts on Tempo EVM; Anchor program on Solana; Chainlink CCIP for cross-chain provider registry + `$GAZE`.

## Identity

From this machine, commits and pushes go out **only** as `oskarpetri <op@stargazempp.com>` — enforced by local `.git/hooks/pre-commit` and `pre-push` (not shared via the repo). Pushes are routed through the `github.com-stargazempp` SSH alias.

Other collaborators (the external dev included) commit from their own machines under their own GitHub identities and don't inherit this guard.

## Status

Pre-implementation. Repo skeleton + scope split + external dev brief only.
