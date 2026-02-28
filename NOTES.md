# StargazeMPP — notes

Rough vision draft. Will turn into proper docs once MPP details are public.

## What this is

Marketplace for AI agents to discover and pay for intelligence services. Built on the upcoming MPP (HTTP 402 + Tempo + Stripe). Four layers:

- **StargazeIndex** — public directory of providers and their MPP endpoints.
- **StargazeSession** — MPP session manager, voucher batch settlement (sub-10ms verify, no RPC per voucher).
- **StargazeVault** — ZK privacy wrapper for sensitive intelligence (Groth16 to start).
- **StargazeReputation** — on-chain trust score per provider, slashable.

Token: `$GAZE`. Coordination only — stake / governance / access. Payment rails are PathUSD (Tempo) and USDC (Solana, x402). `$GAZE` is **not** payment.

## Open

- Tempo testnet target / RPC TBD until MPP launches.
- Privacy tier vocabulary not yet fixed (camelCase vs hyphenated TBD).
- Reputation scoring algorithm details TBD — leaning crowd-vote + uptime probe + AI quality check.
- Provider categories: on-chain analytics, physical-AI telemetry, DeSci, RWA/macro, compliance, AI models. Settle on the exact enum before contracts land.
