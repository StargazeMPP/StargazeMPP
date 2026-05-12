# `@stargazempp/frontend`

The StargazeMPP marketplace front-end.

**Stack.** Next.js 16 (App Router, Turbopack, RSC) · React 19.2 · TypeScript 5 · Tailwind CSS 4 · Fraunces / Inter Tight / JetBrains Mono via `next/font/google`.

## Routes

| Route | Purpose |
|---|---|
| `/` | Marketing landing — full brand design, video hero, MPP value prop, FAQ, build CTA. |
| `/dashboard` | Logged-in agent surface: overview, providers table, playground, sessions, stake, reputation. Wallet connect for Phantom + Solflare. |
| `/docs` | Developer docs — x402 + vouchers, sessions, rails, reputation, SDK quickstart. |
| `/privacy` | Privacy policy. |

The marketing landing lives in [`app/_landing.html`](app/_landing.html) and is rendered through [`app/_landing-hydrate.tsx`](app/_landing-hydrate.tsx) so designers can iterate on the HTML directly while the React app handles routing, fonts, and metadata.

## Brand tokens

Defined in [`app/globals.css`](app/globals.css) under `@theme inline`. Updating a token retunes every page that imports them.

## Develop

```bash
npm install
npm run dev
# → http://localhost:3000
```

## Build & test

```bash
npm run build
npm run typecheck
npm run lint
```

## Planned scope

- Wallet adapters (wagmi + viem for Tempo; `@solana/wallet-adapter` for Solana).
- EIP-712 voucher signing helper (`lib/voucher/sign.ts`).
- TanStack Query for server state, Zustand for session state.
- React Hook Form + Zod schemas at every form boundary.
- Recharts for reputation / spend visualisations.
- TanStack Table for sessions + queries lists.
- Sentry + PostHog for observability.
- Playwright e2e suite with mocked wallets.
