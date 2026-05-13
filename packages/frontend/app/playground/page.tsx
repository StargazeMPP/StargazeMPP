import type { Metadata } from "next";
import { Placeholder } from "@/components/placeholder";

export const metadata: Metadata = {
  title: "Playground",
  description:
    "Build and execute real MPP queries — pick a provider, open a session, sign EIP-712 vouchers, inspect responses.",
};

export default function PlaygroundPage() {
  return (
    <Placeholder
      eyebrow="Playground"
      title={
        <>
          Build <em style={{ color: "var(--accent)" }}>real queries</em>, sign real vouchers.
        </>
      }
      description="Three-panel workspace: provider selector + schema-driven request builder on the left, live session monitor (escrow / cumulative / nonce) in the middle, pretty-printed response viewer on the right."
      roadmap={[
        "Wallet-gated entry — Phantom / Solflare / WalletConnect.",
        "Zod-schema-derived form generator from each provider's `requestSchema`.",
        "Real `executeQuery` calls direct to the gateway (skipping Next API).",
        "Auto-incrementing cumulative voucher signing on the Solana voucher schema.",
        "Top-up flow when escrow falls below the request cost.",
      ]}
      cta={{ label: "Use the dashboard playground tab today", href: "/dashboard#playground" }}
    />
  );
}
