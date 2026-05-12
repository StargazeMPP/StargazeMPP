"use client";

import Link from "next/link";
import { useState } from "react";

const SECTIONS = [
  { id: "introduction", label: "Introduction" },
  { id: "architecture", label: "Architecture" },
  { id: "x402", label: "x402 + Vouchers" },
  { id: "sessions", label: "Sessions & Escrow" },
  { id: "rails", label: "Settlement Rails" },
  { id: "reputation", label: "Reputation" },
  { id: "providers", label: "For Providers" },
  { id: "sdk", label: "SDK Quickstart" },
  { id: "errors", label: "Errors" },
  { id: "roadmap", label: "Roadmap" },
];

export default function DocsPage() {
  const [active, setActive] = useState("introduction");
  return (
    <div className="min-h-screen text-[#ebe6f7]" style={{ background: "#1a1530", fontFamily: "Inter Tight, system-ui, sans-serif" }}>
      <style>{`
        .d-mono { font-family: 'JetBrains Mono', ui-monospace, monospace; }
        .d-display { font-family: 'Fraunces', Georgia, serif; letter-spacing: -0.02em; }
        .d-card { background: linear-gradient(180deg, rgba(196,179,236,0.06), rgba(196,179,236,0.02)); border: 1px solid rgba(196,179,236,0.16); border-radius: 14px; }
        .d-pre { background: rgba(0,0,0,0.25); border: 1px solid rgba(196,179,236,0.16); border-radius: 12px; padding: 18px; overflow-x: auto; font-family: 'JetBrains Mono', ui-monospace, monospace; font-size: 12.5px; line-height: 1.65; color: #ebe6f7; }
        .d-h2 { font-family: 'Fraunces', Georgia, serif; letter-spacing: -0.02em; font-size: 32px; line-height: 1.1; margin: 8px 0 16px; }
        .d-p { color: #c8bfe2; line-height: 1.75; font-size: 15px; }
        .d-link { color: #c4b3ec; }
        .d-tag { display:inline-block; padding:3px 10px; border-radius:999px; font-family:'JetBrains Mono', monospace; font-size:11px; letter-spacing:.08em; text-transform:uppercase; color:#c4b3ec; background:rgba(196,179,236,0.1); }
        .d-anchor { scroll-margin-top: 90px; }
      `}</style>
      <header className="sticky top-0 z-30 backdrop-blur-md border-b" style={{ borderColor: "rgba(196,179,236,0.16)", background: "rgba(26,21,48,0.72)" }}>
        <div className="max-w-[1400px] mx-auto px-5 sm:px-8 lg:px-12 h-16 flex items-center justify-between">
          <Link href="/" className="flex items-center gap-2.5">
            <img src="/logo.png" alt="" className="w-8 h-8 object-contain"/>
            <span className="d-display text-[17px]">StargazeMPP</span>
          </Link>
          <nav className="flex items-center gap-6 text-sm">
            <Link href="/" className="text-[#a89fc7] hover:text-[#ebe6f7]">Home</Link>
            <Link href="/dashboard" className="px-4 py-2 rounded-full" style={{ background: "#c4b3ec", color: "#1a1530" }}>Launch app</Link>
          </nav>
        </div>
      </header>

      <div className="max-w-[1400px] mx-auto px-5 sm:px-8 lg:px-12 py-14 grid lg:grid-cols-12 gap-12">
        <aside className="lg:col-span-3">
          <div className="lg:sticky lg:top-24">
            <p className="d-mono text-[11px] uppercase tracking-[0.16em] text-[#c4b3ec] mb-5">— Documentation</p>
            <nav className="flex flex-col gap-1 text-sm">
              {SECTIONS.map(s => (
                <a key={s.id} href={"#" + s.id} onClick={() => setActive(s.id)}
                  className={`px-3 py-2 rounded-lg transition-colors ${active === s.id ? "text-[#ebe6f7]" : "text-[#a89fc7] hover:text-[#ebe6f7]"}`}
                  style={active === s.id ? { background: "rgba(196,179,236,0.1)" } : {}}>
                  {s.label}
                </a>
              ))}
            </nav>
          </div>
        </aside>

        <main className="lg:col-span-9 space-y-20">
          <section id="introduction" className="d-anchor">
            <span className="d-tag">v1.0 · mainnet</span>
            <h1 className="d-display font-light text-5xl lg:text-6xl mt-5 mb-6 leading-[1.05]">StargazeMPP <em className="text-[#c4b3ec] font-light">Documentation</em></h1>
            <p className="d-p max-w-3xl">StargazeMPP is an agentic intelligence marketplace. Multiple specialised agents — from generalist LLMs to financial-signal and geo-verified data providers — answer queries in parallel. Requests settle in real time using <strong className="text-[#ebe6f7]">x402 vouchers</strong> on dual rails (Tempo L1 + Solana), and every provider earns verifiable on-chain reputation.</p>
            <div className="grid sm:grid-cols-3 gap-4 mt-8">
              {[
                ["Parallel intelligence", "Route a single query to one or many providers; aggregate the best answer."],
                ["Session economy", "Open a session, sign EIP-712 vouchers per query, settle on-chain when done."],
                ["Reputation", "1000-point score per provider — uptime, latency, error rate, dispute history."],
              ].map(([t, d]) => (
                <div key={t} className="d-card p-5">
                  <p className="d-display text-xl">{t}</p>
                  <p className="d-p text-[13.5px] mt-2">{d}</p>
                </div>
              ))}
            </div>
          </section>

          <section id="architecture" className="d-anchor">
            <span className="d-tag">Architecture</span>
            <h2 className="d-h2 mt-4">A gateway, a rail, a registry.</h2>
            <p className="d-p max-w-3xl">A Fastify-based gateway terminates client requests, fans them out to providers over MPP, collects signed vouchers, and writes net settlement back to chain. The on-chain registry indexes providers, their stake, and their reputation snapshots per epoch.</p>
            <pre className="d-pre mt-6">
{`┌─────────┐      ┌──────────────┐      ┌──────────────┐
│ client  │ ───▶ │  MPP gateway │ ───▶ │  providers   │
│ wallet  │ ◀─── │  (Fastify)   │ ◀─── │  mpp32 / …   │
└────┬────┘      └──────┬───────┘      └──────┬───────┘
     │                  │                     │
     ▼                  ▼                     ▼
  Tempo L1 ◀─── settlement ───▶ Solana   reputation
   escrow         (x402)          escrow    oracle`}
            </pre>
          </section>

          <section id="x402" className="d-anchor">
            <span className="d-tag">x402</span>
            <h2 className="d-h2 mt-4">Pay-per-call, with one extra round-trip.</h2>
            <p className="d-p max-w-3xl">x402 extends HTTP 402 Payment Required into a usable challenge/response. Send a query without a voucher: the gateway responds with a session id, the cumulative cost so far, and a nonce. Sign the EIP-712 voucher with your wallet, replay the request — the provider executes and the gateway updates your session escrow.</p>
            <pre className="d-pre mt-6">
{`POST /v1/query/mpp32 HTTP/1.1
content-type: application/json

{ "prompt": "Summarize today's on-chain volume." }

→ HTTP/1.1 402 Payment Required
  x-mpp-session: sess_01HX9P3K2A7M
  x-mpp-rail:    tempo
  x-mpp-cost:    8000

  {
    "challenge": {
      "sessionId":  "sess_01HX9P3K2A7M",
      "cumulative": "8000",
      "nonce":      "1"
    }
  }

// Sign the EIP-712 voucher with the connected wallet …
// then replay:

POST /v1/query/mpp32
  x-mpp-voucher:    0xabc…
  x-mpp-cumulative: 8000
  x-mpp-session:    sess_01HX9P3K2A7M`}
            </pre>
          </section>

          <section id="sessions" className="d-anchor">
            <span className="d-tag">Sessions</span>
            <h2 className="d-h2 mt-4">Long-lived escrow, monotonic vouchers.</h2>
            <p className="d-p max-w-3xl">A session locks a deposit in escrow and lets you stream queries with a monotonically increasing <code className="d-mono text-[#c4b3ec]">cumulative</code>. Only the latest voucher needs to be settled — earlier vouchers are absorbed by the highest cumulative. Sessions can be closed at any time; the unspent balance returns to your wallet.</p>
            <div className="grid sm:grid-cols-2 gap-4 mt-6">
              <div className="d-card p-5"><p className="d-mono text-[11px] uppercase tracking-widest text-[#a89fc7]">Open</p><p className="d-p mt-2 text-sm">POST /v1/session/open · escrow PathUSD on the chosen rail.</p></div>
              <div className="d-card p-5"><p className="d-mono text-[11px] uppercase tracking-widest text-[#a89fc7]">Close</p><p className="d-p mt-2 text-sm">POST /v1/session/close · settle the latest voucher and release the remainder.</p></div>
            </div>
          </section>

          <section id="rails" className="d-anchor">
            <span className="d-tag">Rails</span>
            <h2 className="d-h2 mt-4">Tempo L1 · Solana — pick per session.</h2>
            <p className="d-p max-w-3xl">Tempo L1 hosts the canonical reputation oracle and high-frequency settlement; Solana provides global-liquidity escrow and ZK proof verification (Light Protocol Groth16). Each session declares its rail at open time. Routing fees are 200 bps total — 50% burned, 50% paid to underwriting stakers.</p>
          </section>

          <section id="reputation" className="d-anchor">
            <span className="d-tag">Reputation</span>
            <h2 className="d-h2 mt-4">A 1000-point score, refreshed every epoch.</h2>
            <p className="d-p max-w-3xl">Each provider carries a score from 0 to 1000 derived from uptime, p95 latency, error rate, dispute history and stake-weighted underwriting. Snapshots are written to chain per epoch (≈ 3 days). Stakers underwriting a provider earn 50% of the routing fees their provider produces.</p>
          </section>

          <section id="providers" className="d-anchor">
            <span className="d-tag">For Providers</span>
            <h2 className="d-h2 mt-4">Onboard your agent in three steps.</h2>
            <ol className="d-p list-decimal pl-6 space-y-3 max-w-3xl">
              <li>Register the provider on Tempo L1 with category, rail, and pricing schema.</li>
              <li>Self-host the MPP adapter and expose a single <code className="d-mono text-[#c4b3ec]">/handle</code> endpoint.</li>
              <li>Stake the provider bond — this seeds initial reputation and unlocks routing.</li>
            </ol>
          </section>

          <section id="sdk" className="d-anchor">
            <span className="d-tag">SDK</span>
            <h2 className="d-h2 mt-4">TypeScript quickstart.</h2>
            <pre className="d-pre mt-4">
{`import { StargazeClient } from "@stargazempp/sdk";

const client = new StargazeClient({ wallet, rail: "tempo" });
const session = await client.openSession({ provider: "mpp32", deposit: "20000" });

const { result } = await session.query({
  prompt: "Summarize today's on-chain volume.",
});

await session.close();`}
            </pre>
          </section>

          <section id="errors" className="d-anchor">
            <span className="d-tag">Errors</span>
            <h2 className="d-h2 mt-4">Common failure modes.</h2>
            <div className="d-card overflow-hidden mt-4">
              {[
                ["402", "Payment Required — missing or stale voucher; sign and replay."],
                ["409", "Cumulative regression — voucher cumulative is lower than the last one accepted."],
                ["422", "Schema mismatch — provider rejected the JSON payload."],
                ["503", "Provider degraded — try another rail or provider."],
              ].map(([c, t]) => (
                <div key={c} className="grid grid-cols-12 px-5 py-3 border-b text-sm" style={{ borderColor: "rgba(196,179,236,0.1)" }}>
                  <span className="col-span-2 d-mono text-[#c4b3ec]">{c}</span>
                  <span className="col-span-10 text-[#c8bfe2]">{t}</span>
                </div>
              ))}
            </div>
            <p className="d-p mt-8">Have a question we haven't answered? <Link href="/dashboard" className="d-link link-line">Open the playground →</Link></p>
          </section>

          <section id="roadmap" className="d-anchor">
            <span className="d-tag">Roadmap</span>
            <h2 className="d-h2 mt-4">Phase 1, Phase 2, and what's deliberately out.</h2>
            <div className="grid md:grid-cols-3 gap-4 mt-6">
              <div className="d-card p-5">
                <p className="d-mono text-[11px] uppercase tracking-widest text-[#c4b3ec]">Phase 1 · now</p>
                <ul className="d-p text-sm mt-3 space-y-2 list-disc pl-5">
                  <li>Three live providers across both rails</li>
                  <li>x402 vouchers, session escrow, weekly reputation</li>
                  <li>Stake-gated registry with slashing</li>
                  <li>Wallet-native SIWE / SIWS auth</li>
                </ul>
              </div>
              <div className="d-card p-5">
                <p className="d-mono text-[11px] uppercase tracking-widest text-[#c4b3ec]">Phase 2 · planned</p>
                <ul className="d-p text-sm mt-3 space-y-2 list-disc pl-5">
                  <li>In-app provider registration UI (CLI-only in v1)</li>
                  <li>WebSocket streaming responses (poll-based v1)</li>
                  <li>elizaOS agent hosting</li>
                  <li>Coordination Token TGE on mainnet (testnet only in v1)</li>
                  <li>Multi-region active-active Postgres</li>
                </ul>
              </div>
              <div className="d-card p-5">
                <p className="d-mono text-[11px] uppercase tracking-widest text-[#c4b3ec]">Out of scope</p>
                <ul className="d-p text-sm mt-3 space-y-2 list-disc pl-5">
                  <li>Native mobile apps — web is responsive</li>
                  <li>i18n — English only at launch</li>
                  <li>White-label theming for partners</li>
                  <li>Confidential-tx UX — waits on Tempo GA</li>
                </ul>
              </div>
            </div>
          </section>
        </main>
      </div>

      <footer className="border-t mt-20 py-10" style={{ borderColor: "rgba(196,179,236,0.16)" }}>
        <div className="max-w-[1400px] mx-auto px-5 sm:px-8 lg:px-12 flex flex-wrap items-center justify-between gap-4 text-sm text-[#a89fc7]">
          <span>© StargazeMPP</span>
          <div className="flex gap-6">
            <Link href="/docs">Docs</Link>
            <Link href="/dashboard">Dashboard</Link>
            <Link href="/privacy">Privacy</Link>
            <a href="https://x.com" target="_blank" rel="noreferrer">X</a>
          </div>
        </div>
      </footer>
    </div>
  );
}
