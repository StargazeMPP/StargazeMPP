# StargazeMPP — external dev brief

Hand this doc to your Claude Code session as the first message. It is self-contained: nothing in here assumes you've read the PDFs or talked to the internal team. Both PDFs that ground the spec are at `docs/overview.pdf` (10pp) and `docs/backend.pdf` (10pp); read them once before starting.

---

## 1. What StargazeMPP is

The **first agentic intelligence marketplace built natively on the Machine Payments Protocol** (MPP — Stripe + Tempo, IETF Internet-Draft, HTTP 402 reborn for machines). AI agents discover, access, and pay for on-chain intelligence in a single HTTP request cycle: agent hits a provider endpoint, server returns `402 Payment Required` with price + accepted methods, agent wallet signs an EIP-712 voucher, agent retries with the voucher as a header, server verifies via `ecrecover` (no RPC, no database lookup), delivers the resource plus a receipt. Session mode lets an agent pre-fund escrow once and stream signed cumulative vouchers to multiple providers, with a single batch settlement on close.

StargazeMPP layers four services on top of the MPP primitive:

- **StargazeIndex** — public provider directory at `/stargaze.json`.
- **StargazeSession** — dual-rail (Tempo PathUSD + Solana USDC x402) session manager.
- **StargazeVault** — ZK privacy wrapper for sensitive intelligence (Groth16, snarkjs).
- **StargazeReputation** — on-chain trust score per provider; bad responses slash `$GAZE` stake.

`$GAZE` is the coordination token (stake / governance / access — **not** payment). Payment is PathUSD on Tempo or USDC on Solana, with Stripe / Visa / Lightning as fiat on-ramps.

Categories of intelligence the marketplace serves: on-chain analytics, physical-AI telemetry (drones, robots), DeSci research, RWA + macro signals, compliance intel, AI model endpoints.

---

## 2. Your scope

You own everything that is **not** a chain, a contract, a token, a voucher signature, or a ZK proof. Concretely:

### Backend (`packages/backend`)

1. **API gateway** — Express 5 + tRPC 11. Wallet-signed JWT for session-scoped auth. CORS, rate limiting, structured logging that **never** stores PII (wallet address is the only identity).
2. **StargazeIndex Service** — provider registry CRUD, category management, full-text search, public `/stargaze.json` directory endpoint, live stats endpoint.
3. **MPP Session Manager (orchestration)** — `openSession` / `validateVoucher` / `settleSession` lifecycle from `docs/backend.pdf` §3. Voucher validation must be sub-10ms (no RPC, no DB per validation — Redis lookup only). Sessions persist in Redis with 24h TTL. Settlement triggers on close OR when balance falls below 10% of deposit. The cryptographic primitive (`ecrecover`-based EIP-712 verification) is provided by the internal team via `packages/provider-sdk` / `packages/shared` — you wire it into the lifecycle.
4. **Reputation Oracle** — elizaOS v2 long-running agent process. Four functions: synthetic latency probe (5 test queries / provider / hour, p50/p95/p99 tracking), accuracy vote aggregator (reads on-chain votes from `StargazeRegistry`), AI quality assessor (Claude API, only triggers above complaint threshold), slash proposer (writes a DAO proposal when fraud is confirmed). Outputs signed and committed on-chain by the internal team's payment router.
5. **Database** — PostgreSQL + TimescaleDB. Drizzle ORM. The three core tables from `docs/backend.pdf` §6 (`providers`, `sessions`, `queries`) plus the implied auxiliaries (`reputation_events`, `gaze_burns`, `vault_proofs`, `provider_categories`). TimescaleDB hypertables on `registered_at` / `opened_at` / `executed_at`. Migrations. Schema lives in `packages/shared/db/schema.ts` so the indexer can read it.
6. **Cache layer** — Redis (Upstash). Session state, voucher queue, provider lookup, rate limit counters.
7. **tRPC routes** (full list in `docs/backend.pdf` §7) — `index.*`, `session.*`, `provider.*`, `vault.*`, `reputation.*`, `gaze.*`.

### Provider SDK ergonomics (`packages/provider-sdk`)

Build the developer-facing API: `new StargazeProvider({...})`, `provider.monetize(handler)`, `provider.vaultMonetize(circuitConfig, handler)`. Config validation, error surface, working examples for Express / Hono / Fastify, README, npm publish flow. The internal team plugs in the voucher-verification internals via a clearly typed interface in `packages/shared` (`MppVerifier`, `VaultProofGenerator`).

### Frontend (`packages/frontend`)

Next.js 15. Deep-space / star-violet aesthetic. Pages:

- `/` — marketing landing per `docs/overview.pdf`.
- `/index` — StargazeIndex browser (filter by category / price / privacy / reputation / payment method, full-text search).
- `/providers/[id]` — provider detail (pricing, privacy tier, sample response, reputation breakdown, recent queries).
- `/agent` — agent session dashboard (open session, balance, spend, query log, close + refund).
- `/provider` — provider onboarding (wallet connect, service config, `$GAZE` stake, ZK verifier spec upload for VAULT tier, dashboard with earnings + reputation + query analytics).
- `/gaze` — live `$GAZE` burn / stake / staking-APY feed.

### Deployment + infra

- `docker-compose.yml` (Postgres + TimescaleDB + Redis).
- Railway / Fly.io for API + Reputation Oracle (elizaOS v2 needs a persistent process).
- Vercel for frontend.
- Neon / Supabase for managed Postgres.
- Upstash for Redis.
- Irys (Bundlr) for Arweave ZK proof permanent storage.
- PagerDuty wiring for Reputation Oracle anomalies, settlement failures, provider slashes.
- `.env.example`, bootstrap docs, `bun run dev` story.

### What is **not** yours (internal team owns)

- All Tempo EVM contracts (`packages/contracts-evm`).
- The Solana `StargazeAnchor` program (`packages/anchor-program`).
- The Solana indexer (`packages/indexer`).
- Groth16 circuits + on-chain verifiers (`packages/vault-circuits`).
- The `ecrecover` voucher-verification primitive (you call it; we write it).
- `$GAZE` tokenomics, burn execution, staker distribution on-chain.
- Chainlink CCIP cross-chain bridge.
- Trail of Bits + Immunefi coordination.

If a task requires touching any of the above, file a question in `BLOCKERS.md` and ping the internal team — do **not** write a placeholder Solidity / Rust / circom file.

---

## 3. Onboarding

Ask the internal team for collaborator access on `github.com/StargazeMPP/StargazeMPP`. Once granted:

```bash
git clone git@github.com:StargazeMPP/StargazeMPP.git
cd StargazeMPP

# Use your own GitHub identity — set per-repo so it doesn't leak into other projects
git config user.name  "<your-github-username>"
git config user.email "<your-github-noreply-or-real-email>"

bun install                           # workspaces
cd docker && docker compose up -d     # Postgres + TimescaleDB + Redis (once docker-compose.yml exists — first task)
cd ../packages/backend && cp .env.example .env
# fill TEMPO_RPC, ANTHROPIC_API_KEY, HELIUS_API_KEY, contract addresses (from shared/evm/abi/deployed.json once internal team publishes)
bun run db:push
bun run dev
```

There is no identity enforcement on your machine — the internal team runs a local pre-commit / pre-push guard on their own clone to keep our maintainer identity single, but the repo itself does not ship it. Commit as yourself; PR-review is the source of truth for what lands.

---

## 4. Boundary contract — `packages/shared`

This is the only place your code touches the internal team's code. Treat it as a public API both sides agree on.

| Path | Owner | Consumer | Purpose |
|---|---|---|---|
| `evm/abi/*.json` | internal | backend | Tempo contract ABIs (typed via `viem` or `ethers` — your call) |
| `evm/addresses.ts` | internal | backend | Per-network deployed addresses |
| `solana/idl/stargaze_anchor.json` | internal | backend, indexer | Anchor IDL |
| `mpp/voucher.ts` | both | both | EIP-712 typed-data definition |
| `mpp/session.ts` | both | both | JWT claims shape (`agentWallet`, `sessionId`, `spendingLimit`, `expiry`) |
| `mpp/verifier.ts` | internal | backend, provider-sdk | `MppVerifier` interface — `verifyVoucher(v) → { agentWallet, amount } \| Error` |
| `vault/verifier-bundle.ts` | internal | backend | Groth16 verifying-key + public-output schema per tier |
| `vault/proof-generator.ts` | internal | backend, provider-sdk | `VaultProofGenerator` interface — `generate(circuit, privateInputs, publicParams) → { publicOutput, proofHash, proof }` |
| `db/schema.ts` | you | indexer | Drizzle schema source of truth |
| `categories.ts` | you | both | Provider category enum |

If you need something from this list that isn't there yet, stub the type with a TODO and open a one-line note in `BLOCKERS.md`. Don't reimplement.

---

## 5. Milestones — mapped to the overview PDF roadmap

The overview PDF has a 5-phase roadmap. Your work concentrates in phases 1–3.

### M1 — StargazeIndex + Session Infrastructure (weeks 1–8)

Goal: public directory live; ten providers indexed; sessions can open / query / close / settle end-to-end on Tempo testnet with the internal team's testnet contract addresses.

- [ ] Monorepo bootstrap: `docker-compose.yml`, `.env.example`, `bun run dev` working.
- [ ] Drizzle schema for `providers`, `sessions`, `queries`. TimescaleDB hypertables. Migration runner.
- [ ] tRPC server scaffold with wallet-bound JWT auth middleware.
- [ ] `index.*` routes: `getProviders` (with category / price / privacy / reputation / method filters), `getProvider`, `search`, `getLiveStats`. Plus the `/stargaze.json` public directory endpoint.
- [ ] `provider.*` routes: `register`, `update`, `getDashboard`, `withdraw`.
- [ ] `session.*` routes: `open`, `query`, `close`, `getStatus`. Voucher validation via the `MppVerifier` interface from `shared`.
- [ ] Redis session state (24h TTL) + voucher queue.
- [ ] Settlement trigger: balance threshold OR explicit close.
- [ ] Provider SDK v0: `monetize(handler)` decorator, `serviceId` / `category` / `pricing` / `methods` config, Express example, README, npm publish dry-run.
- [ ] Frontend: landing + `/index` browser + `/providers/[id]` detail. Hook into tRPC.
- [ ] Health endpoint, structured logging, request tracing.
- [ ] Deploy preview: Railway (API) + Vercel (frontend) + Neon (Postgres) + Upstash (Redis).

Exit criteria: an agent can open a Tempo session, query MPP32 + TrigonFi + Kalder + AxonMed-HRV providers from the index, and close with correct settlement. Ten providers registered. Frontend shows them all.

### M2 — StargazeVault + Physical AI data (weeks 9–18)

Goal: ZK privacy wrapper live; AirborneLabs + YaloBase data NFTs registered as MPP services; 50+ providers; 10k queries/day.

- [ ] `vault.*` routes: `configure`, `testProof`, `getSpec`. Proxy proof generation through the `VaultProofGenerator` interface.
- [ ] Provider SDK v1: `vaultMonetize(circuitConfig, handler)` decorator.
- [ ] Frontend: provider onboarding ZK config flow + agent-side proof-verification UI.
- [ ] Reputation Oracle v0 (elizaOS v2): latency probe + accuracy aggregator running in production.
- [ ] `reputation.*` routes: `vote` (with `$GAZE` burn ix call), `getHistory`, `getScore`.
- [ ] Arweave (Irys) integration for permanent proof storage.
- [ ] Per-buyer key envelope flow for BUYER-KEY tier (physical-AI data NFTs).
- [ ] Frontend: agent session dashboard `/agent` with proof verification UI.

Exit criteria: an agent can query a private AxonMed cohort and verify the returned Groth16 proof in-browser without seeing underlying data. Drone / robot operators are earning PathUSD per query.

### M3 — Audit + TGE + AI Model Marketplace (weeks 19–26)

Goal: Trail of Bits audit closed; `$GAZE` TGE; AI model endpoints (GroundIntel, AeroIntel) listed; 100+ providers; 100k queries/day.

- [ ] Reputation Oracle v1: AI quality assessor (Claude API on complaint threshold) + slash proposer.
- [ ] `gaze.*` routes: `getStats`, `getBurnFeed`, `stake`, `unstake`, `claimRewards`. Burn-feed websocket / SSE.
- [ ] Frontend: `/gaze` live burn / stake / APY page.
- [ ] PagerDuty alerting wired for Oracle anomalies, settlement failures, slashes.
- [ ] Pen-test scope handoff to Trail of Bits for the backend perimeter (the contracts are scoped separately).
- [ ] Bug bounty: Immunefi listing for the backend / SDK (the contracts side is internal team's).

### M4–M5 — Enterprise + CCIP + full economy (weeks 27+)

Mostly internal-team-driven (cross-chain registry, enterprise bulk sessions). Your role: support the CCIP-mirrored registry in the index UI, support enterprise bulk session UX.

---

## 6. Task list — actionable items for week 1

These are sized so each is a single PR. Open one branch per task, target `main`.

1. `chore/docker-compose` — `docker/docker-compose.yml` for Postgres 16 + TimescaleDB extension + Redis 7. README in `docker/` explains `docker compose up -d` flow.
2. `chore/env-example` — `packages/backend/.env.example` with every variable from `docs/backend.pdf` §9, commented. Document which are required vs optional.
3. `feat/shared-schema` — `packages/shared/db/schema.ts` Drizzle schema for the three core tables. Hypertable migration SQL in `packages/shared/db/migrations/0001_init.sql`. Export from `packages/shared/index.ts`.
4. `feat/backend-bootstrap` — Express 5 + tRPC 11 + Drizzle wiring in `packages/backend`. Health endpoint at `/health`. Structured logging. CORS from `CORS_ORIGIN`.
5. `feat/index-routes` — `index.getProviders` / `getProvider` / `search` / `getLiveStats` + the `/stargaze.json` directory endpoint. Public, no auth. Filters as listed in `docs/backend.pdf` §7.
6. `feat/provider-register` — `provider.register` / `update` / `getDashboard` / `withdraw`. Wallet-signed message auth (use `viem`'s `verifyMessage`). At registration time, accept the `$GAZE` stake tx hash from the client and verify it via the internal team's `StargazeRegistry` ABI call (read-only — registry sees the stake on-chain).
7. `feat/session-open` — `session.open`. Validates the deposit tx hash (call into internal team's `MppVerifier.verifyDeposit`), creates session in Redis with 24h TTL, returns signed JWT session token (`agentWallet` + `sessionId` + `spendingLimit` + `expiry`).
8. `feat/session-query` — `session.query`. Voucher validation via `MppVerifier.verifyVoucher` (sub-10ms target — Redis read only), proxies the request to the provider's MPP endpoint, records the row in `queries`, updates session balance in Redis.
9. `feat/session-close` — `session.close` triggers batch settlement: builds the settle payload from accumulated vouchers and hands off to the internal team's Payment Router (interface in `shared/mpp/payment-router.ts`). Persists `settlement_tx`.
10. `feat/redis-state` — extract the session-state Redis layer behind a typed module; rate-limit middleware; voucher dedupe queue.

Stop at #10 and ship a deploy preview before continuing. Get the internal team to point a Tempo-testnet agent wallet at the deploy and walk a full session through.

---

## 7. Working agreement

- **PRs only.** No direct pushes to `main`. `main` is the source of truth — open a PR, wait for internal team review on anything that touches `packages/shared`.
- **Commit as yourself.** Use your own GitHub identity / SSH key. The internal team's machine has a local maintainer-identity guard; you don't inherit it and don't need to mimic it.
- **Don't reimplement crypto.** If a task description mentions `ecrecover`, EIP-712 verification, Groth16 proof generation, or `$GAZE` burn execution, you are calling into `packages/shared` — not writing it. If the interface isn't there yet, stub the type and add a one-liner to `BLOCKERS.md`.
- **No PII anywhere.** Wallet address is identity. Don't log emails, IPs (beyond rate-limit counters that expire in Redis), or any field a regulator would call PII.
- **Schema migrations are forward-only and reviewed.** Once a hypertable is created in prod, you don't `DROP` it — you migrate.
- **The Reputation Oracle is a persistent process, not a cron.** elizaOS v2 needs to stay running; Railway / Fly.io is what the deploy story expects.
- **Sub-10ms voucher validation is a hard budget.** If your `session.query` p95 climbs past that on a hot session, that's a P0.

---

## 8. Open questions for the internal team (pre-week-1)

These should be resolved before you start; pull-request `BLOCKERS.md` updates as the answers land.

1. Which Tempo testnet are we targeting first? RPC URL + chain ID needed in `.env.example`.
2. When will `packages/shared/evm/abi/` ship with at least placeholder ABIs for `GAZEToken`, `StargazeEscrow`, `StargazeRegistry`?
3. What's the deposit-tx verification interface? Best-guess: `verifyDeposit(txHash, expectedRecipient, minAmount) → { amount, agentWallet }`. Confirm shape so `session.open` can type it correctly.
4. JWT signing key: rotate via env or KMS? Default to env for testnet.
5. Reputation Oracle — does the on-chain commit happen from the internal team's payment-router wallet, or does the Oracle need its own key?
6. Is the Solana-side session manager scope-shared (i.e., do you also wire `session.query` for x402 USDC), or is that internal team only for M1?

When in doubt, write the question into `BLOCKERS.md` rather than guessing.

---

## 9. Definition of done — repo-wide

A PR is done when:

- Tests for the changed paths exist and pass (`bun test`).
- Types check (`tsc --noEmit`).
- No `as any`, no `// @ts-ignore` without a linked GitHub issue.
- No PII captured in logs (`rg -n "ip|email|name" packages/backend/src` reviewed by hand).
- Migrations idempotent.
- A note in the PR body explaining anything in `packages/shared` you touched.
