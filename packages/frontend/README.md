# `@stargazempp/frontend`

The StargazeMPP marketplace front-end.

**Stack.** TanStack Start (TypeScript) · React · shadcn/ui · Tailwind CSS · Cloudflare Workers (via `@cloudflare/vite-plugin`).

## Surfaces

- `/` — marketing landing.
- `/index` — public StargazeIndex browser (filter by category, price, privacy tier, reputation, payment method).
- `/providers/[id]` — provider detail (pricing, sample response, reputation breakdown).
- `/agent` — agent session dashboard (open session, balance, query log, close + refund).
- `/provider` — provider onboarding (wallet connect, service config, `$GAZE` stake, vault verifier upload, earnings dashboard).
- `/gaze` — live `$GAZE` burn / stake / staking-APY feed.

## Develop

```bash
npm install
npm run dev
```
