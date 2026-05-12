# Scope — 50/50 split

This team handles every line of code that touches a chain, a token, a voucher signature, or a ZK proof. The external dev handles the API surface, the database, the front-end, the developer ergonomics, and the deployment story. The seam is `packages/shared` — types, ABIs, IDL, schemas.

## This team (Solana / Anchor / token / web3)

1. **Tempo EVM contracts** (`packages/contracts-evm`): `GAZEToken`, `BurnController`, `StargazeEscrow`, `StargazeRegistry`, `PrivacyVaultRegistry`. Solidity. Foundry preferred; Hardhat optional for deploy scripts. 4-of-7 Safe multisig + 14-day timelock + Trail of Bits audit + Certora on the transfer hook.
2. **Solana program** (`packages/anchor-program`): `StargazeAnchor` mirror of `StargazeRegistry` for Solana-native providers, CCIP bridge to Tempo.
3. **ZK pipeline** (`packages/vault-circuits`): circom circuits for ZK-AGGREGATE / ZK-GEOFENCE / BUYER-KEY privacy tiers; Groth16 trusted-setup ceremony; on-chain verifiers; integration with the Provider SDK's `vaultMonetize` decorator.
4. **Solana indexer** (`packages/indexer`): Rust + Yellowstone gRPC; emits denormalized projections that the backend reads.
5. **Voucher cryptography** (lives in `packages/provider-sdk` and `packages/shared`): EIP-712 voucher schema, `ecrecover`-based verification, cumulative-amount monotonicity rule, x402 receipt parsing.
6. **Payment routing primitives**: PathUSD ↔ USDC ↔ Stripe / Visa / Lightning rails; `$GAZE` fee burn execution; staker distribution math. The backend's Payment Router calls into these.
7. **`$GAZE` tokenomics**: stake / unstake / cooldown / weekly reward distribution / burn accounting / Verified Provider threshold logic — all enforced on-chain in `GAZEToken` and `StargazeRegistry`.
8. **CCIP integration**: cross-chain `$GAZE` and provider-registry mirroring across Tempo + Solana + Base.
9. **Audit + bug bounty coordination**: Trail of Bits scoping, Immunefi listing ($200K critical / $50K high), Certora spec.

## External dev (backend + frontend + infra + UX)

1. **API gateway** (`packages/backend`): Express 5 + tRPC 11 setup, middleware, wallet-bound JWT session tokens, rate limiting, CORS, request logging that **never** captures PII.
2. **StargazeIndex Service**: provider registry CRUD, category management, full-text search, public `/stargaze.json` directory, live stats.
3. **MPP Session Manager (orchestration)**: open / query / close / status / batch-settle lifecycle on top of the cryptographic primitives this team provides. Sub-10ms voucher validation budget. Redis-backed session state with 24h TTL. Settlement triggers on close OR when balance falls below 10% of deposit.
4. **Reputation Oracle**: elizaOS v2 agent process (latency probe, accuracy aggregator, AI quality assessor via Claude API, slash proposer). Continuous background process. Signed on-chain commits.
5. **Database + ORM** (`packages/shared/db` + backend wiring): Drizzle schema for `providers`, `sessions`, `queries`, plus the other tables implied by the backend PDF §6. TimescaleDB hypertables on `registered_at` / `opened_at` / `executed_at`. Migrations.
6. **Cache layer**: Redis (Upstash) — session state, voucher queue, provider lookup, rate limit counters.
7. **tRPC routes** (all of them — see backend PDF §7): `index.*`, `session.*`, `provider.*`, `vault.*`, `reputation.*`, `gaze.*`.
8. **Provider SDK ergonomics** (`packages/provider-sdk`): the decorator API (`monetize`, `vaultMonetize`), config validation, error surface, examples for Express / Hono / Fastify. This team plugs in the voucher-verification internals.
9. **Frontend** (`packages/frontend`): Next.js 15 marketplace UI per overview PDF §3 and §6 — StargazeIndex browser, provider onboarding, agent session dashboard, `$GAZE` stake / burn live feed.
10. **Deployment**: `docker-compose.yml` for local Postgres + TimescaleDB + Redis. Railway / Fly.io for API + Reputation Oracle. Vercel for frontend. Neon / Supabase for managed Postgres. Upstash for Redis. Irys (Bundlr) for Arweave ZK proof storage. PagerDuty alerting.
11. **Observability**: health endpoint, structured logs, request tracing, Reputation Oracle anomaly alerts, settlement-failure alerts.
12. **Dev environment**: `bun run dev` story; `.env.example`; `db:push` script; bootstrap docs.

## Boundary (`packages/shared`)

Both sides only cross by adding to or reading from `packages/shared`:

- `evm/abi/*.json` — ABIs for the Tempo contracts. This team publishes, backend imports.
- `solana/idl/stargaze_anchor.json` — Anchor IDL. This team publishes, backend + indexer import.
- `mpp/voucher.ts` — EIP-712 typed data definition. Both sides import.
- `mpp/session.ts` — JWT claims shape. Both sides import.
- `vault/verifier-bundle.ts` — Groth16 verifying-key + public-output schema. This team publishes, backend imports for the `vault.*` tRPC routes.
- `db/schema.ts` — Drizzle schema. External dev owns; this team reads for the indexer.
- `categories.ts` — enum.

Rule: if something belongs in `shared` and it isn't there yet, the side that needs it adds the stub and pings the other side.
