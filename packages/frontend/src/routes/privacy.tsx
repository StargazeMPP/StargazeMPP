import { createFileRoute, Link } from "@tanstack/react-router";

export const Route = createFileRoute("/privacy")({
  component: Privacy,
  head: () => ({ meta: [
    { title: "Privacy Policy — StargazeMPP" },
    { name: "description", content: "How StargazeMPP handles wallets, queries, voucher metadata, and on-chain data." },
  ] }),
});

function Privacy() {
  return (
    <div className="min-h-screen text-[#ebe6f7]" style={{ background: "#1a1530", fontFamily: "Inter Tight, system-ui, sans-serif" }}>
      <style>{`
        .p-display { font-family: 'Fraunces', Georgia, serif; letter-spacing: -0.02em; }
        .p-mono { font-family: 'JetBrains Mono', ui-monospace, monospace; }
        .p-p { color: #c8bfe2; line-height: 1.8; font-size: 15px; }
        .p-h2 { font-family: 'Fraunces', Georgia, serif; font-size: 26px; letter-spacing: -0.02em; margin-top: 36px; margin-bottom: 12px; color: #ebe6f7; }
      `}</style>
      <header className="sticky top-0 z-30 backdrop-blur-md border-b" style={{ borderColor: "rgba(196,179,236,0.16)", background: "rgba(26,21,48,0.72)" }}>
        <div className="max-w-[1400px] mx-auto px-5 sm:px-8 lg:px-12 h-16 flex items-center justify-between">
          <Link to="/" className="flex items-center gap-2.5">
            <img src="/logo.png" alt="" className="w-8 h-8 object-contain"/>
            <span className="p-display text-[17px]">StargazeMPP</span>
          </Link>
          <nav className="flex items-center gap-6 text-sm">
            <Link to="/docs" className="text-[#a89fc7] hover:text-[#ebe6f7]">Docs</Link>
            <Link to="/dashboard" className="px-4 py-2 rounded-full" style={{ background: "#c4b3ec", color: "#1a1530" }}>Launch app</Link>
          </nav>
        </div>
      </header>

      <main className="max-w-3xl mx-auto px-5 sm:px-8 py-20">
        <p className="p-mono text-[11px] uppercase tracking-[0.16em] text-[#c4b3ec] mb-4">— Legal · Effective May 2026</p>
        <h1 className="p-display font-light text-5xl lg:text-6xl leading-[1.05]">Privacy <em className="text-[#c4b3ec]">Policy</em></h1>
        <p className="p-p mt-6">StargazeMPP ("we", "us", "the protocol") operates a decentralised marketplace for agentic intelligence. This document explains what data the gateway, the dashboard, and the on-chain settlement contracts process when you use the network.</p>

        <h2 className="p-h2">1. What we collect</h2>
        <p className="p-p">Wallet addresses you choose to connect, the EIP-712 vouchers you sign, session identifiers, the provider you query, and the metadata required to settle a session (rail, cumulative cost, nonce). We do <strong className="text-[#ebe6f7]">not</strong> collect your prompts or provider responses on the gateway — those flow directly between client and provider.</p>

        <h2 className="p-h2">2. On-chain data</h2>
        <p className="p-p">Settlements and reputation snapshots are written to Tempo L1 and Solana. By design, this data is public, immutable, and pseudonymous. Removing on-chain records is technically impossible.</p>

        <h2 className="p-h2">3. Wallet connections</h2>
        <p className="p-p">When you connect Phantom or Solflare, the dashboard receives only your public key and the signatures you explicitly approve. We never request the ability to spend funds without your per-transaction consent.</p>

        <h2 className="p-h2">4. Telemetry</h2>
        <p className="p-p">The gateway records anonymised request counts, p95 latency, and error codes for reputation scoring. No prompt content, no IP-level fingerprints, no third-party trackers.</p>

        <h2 className="p-h2">5. Cookies</h2>
        <p className="p-p">The dashboard uses <span className="p-mono text-[#c4b3ec]">localStorage</span> to remember your preferred wallet and rail. No advertising or analytics cookies.</p>

        <h2 className="p-h2">6. Provider data</h2>
        <p className="p-p">Each provider operates independently. Their handling of prompts and responses is governed by their own published policy, surfaced on their provider page in the dashboard.</p>

        <h2 className="p-h2">7. ZK-verified queries</h2>
        <p className="p-p">For Stargaze Tech providers like <em>stargazempp · geo</em> that use Light Protocol Groth16 proofs, raw inputs (e.g. coordinates) never leave the client. Only the proof and a public commitment reach the provider and the chain.</p>

        <h2 className="p-h2">8. Your rights</h2>
        <p className="p-p">You can disconnect your wallet at any time, clear browser storage, and stop using the network. Off-chain data tied to your wallet (preferences, cached metadata) is removed when you clear storage. On-chain history persists by protocol design.</p>

        <h2 className="p-h2">9. Contact</h2>
        <p className="p-p">Questions about this policy: <a href="https://x.com" target="_blank" rel="noreferrer" className="text-[#c4b3ec]">@stargazempp on X</a>.</p>

        <p className="p-mono text-[11px] uppercase tracking-[0.16em] text-[#a89fc7] mt-16">— End of policy</p>
      </main>

      <footer className="border-t mt-10 py-10" style={{ borderColor: "rgba(196,179,236,0.16)" }}>
        <div className="max-w-[1400px] mx-auto px-5 sm:px-8 lg:px-12 flex flex-wrap items-center justify-between gap-4 text-sm text-[#a89fc7]">
          <span>© StargazeMPP</span>
          <div className="flex gap-6">
            <Link to="/docs">Docs</Link>
            <Link to="/dashboard">Dashboard</Link>
            <Link to="/privacy">Privacy</Link>
            <a href="https://x.com" target="_blank" rel="noreferrer">X</a>
          </div>
        </div>
      </footer>
    </div>
  );
}
