## Goal

Cross-check the landing page (and `/docs`, `/dashboard`, `/privacy`) against the two attached spec PDFs, fix the framework mismatch (the site claims Next.js 16 but actually runs on TanStack Start), and add the few sections the docs justify but that aren't yet on the site.

## Findings from the PDFs vs current site

**Accurate already** (keep as-is): Provider Directory, Agent Playground, Session Escrow, Stake & Earn, Reputation Oracle, Tempo + Solana dual rail, EIP-712 vouchers, x402 dance, Light Protocol ZK proofs, 0–1000 reputation score, 50% burn / 50% staker fee split, Fastify + Drizzle + Postgres + Redis + BullMQ backend, Solidity contracts on Tempo L1.

**Wrong** — fix on this pass:
- `src/site.html` line 546 and 552 say *"A Next.js 16 frontend…"* and *"Next.js 16 · App Router"*. The actual app uses **TanStack Start (React 19, Vite 7)**. Replace these two strings only — the rest of the FE stack bullets (TS strict, Tailwind 4, shadcn, lucide, TanStack Query, wagmi/viem, Solana wallet adapter, Vitest/Playwright, < 200KB/route) match both the PDF and our codebase, so they stay.
- The "Frontend" trace card body should reference TanStack Start + Vite + file-based routing instead of "App Router / RSC".

**Missing sections worth adding** (each is explicitly in the PDFs and would make the site more complete without bloat):

1. **Network Stats strip** (PDF FE §4.1 Landing): live counters — Total providers, Queries (24h), Volume 24h (PathUSD), Avg p95 latency. Static demo numbers, placed right under the hero / above Section 01.
2. **Token Economics card** inside Section 03 (PDF BE §3 Fees + §8.1 Coordination Token): a small 3-up panel showing Routing fee 200 bps, Burn share 50%, Staker share 50%, plus "no mint after deploy" note. Slots in next to the existing Coordination Token / Registry / Escrow / Oracle row.
3. **Security & Trust** new section between Reputation (Section 05 area) and Providers: stake-gated registration, slashing on misbehavior, ZK-verified geo proofs, EIP-712 / SIWS auth, Sentry + OTel observability. Pulled from BE §7.4, §8.2, §10.
4. **Roadmap / Out of Scope** block inside the Docs page (FE §19 + BE §18): Phase 1 vs Phase 2 (in-app provider registration UI, WebSocket streaming, elizaOS agent hosting, $GAZE TGE on mainnet, multi-region active-active Postgres).
5. **FAQ additions** — extend the existing FAQ with three questions the docs answer but the site currently doesn't:
   - "Why two rails (Tempo and Solana)?"
   - "What happens if a provider returns a 5xx?" (gateway refund)
   - "How is reputation computed?" (latency p95, error rate, uptime → 0–1000)

**Intentionally not adding** (out of scope per PDFs or already covered): mobile native apps, i18n, white-label theming, in-app provider registration UX, real-time WebSockets, confidential-tx UX.

## Files to change

- `src/site.html`
  - Replace two "Next.js 16" strings → "TanStack Start" / "TanStack Start · Vite 7".
  - Update Frontend trace card bullet wording (RSC/App Router → file-based routing, server functions).
  - Add Network Stats strip just above Section 01.
  - Add Token Economics 3-up inside Section 03.
  - Add new Security & Trust section between current 05 (Reputation) and 06 (FAQ); renumber subsequent section labels.
  - Append three FAQ items.
- `src/routes/docs.tsx`
  - Add a "Roadmap" section to the SECTIONS sidebar + a matching `<section id="roadmap">` with Phase 1 / Phase 2 / Out of Scope content.
  - Soften the "Fastify-based gateway" paragraph to also mention BullMQ workers, Drizzle, Redis (already partly there — minor polish).
- `src/routes/dashboard.tsx`
  - No structural changes; just sanity-check that no copy says "Next.js" (it doesn't).

## Technical notes

- All edits are pure HTML/JSX content + Tailwind classes; no new deps, no route additions, no backend work.
- Existing cosmos background, fonts, tokens, and section numbering scheme are preserved. New sections reuse `.reveal`, `.trace-card`, and existing typography classes so nothing visual drifts.
- The added stats are presentational (static numbers) — labelled as illustrative; no live data wiring is in scope here.
