import type { Metadata } from "next";
import { Placeholder } from "@/components/placeholder";

export const metadata: Metadata = {
  title: "Sessions",
  description:
    "Past and active MPP sessions for the connected agent wallet — open/close, per-query log, settlement status.",
};

export default function SessionsPage() {
  return (
    <Placeholder
      eyebrow="Session history"
      title={
        <>
          Every <em style={{ color: "var(--accent)" }}>session</em>, every voucher.
        </>
      }
      description="A flat ledger of every session this wallet has opened on StargazeMPP. Row-click reveals the per-query voucher log, latency, and provider used. Open sessions can be closed and force-settled inline."
      roadmap={[
        "TanStack Table with cursor pagination via `GET /v1/agents/me/sessions`.",
        "Inline drawer showing per-query voucher signatures + status.",
        "Settlement transaction link to the appropriate explorer.",
        "Bulk-close action for sessions that have gone idle.",
      ]}
      cta={{ label: "Open the dashboard", href: "/dashboard#sessions" }}
    />
  );
}
