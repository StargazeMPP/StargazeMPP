import type { Metadata } from "next";
import { Placeholder } from "@/components/placeholder";

export const metadata: Metadata = {
  title: "Provider operator dashboard",
  description:
    "Earnings, reputation, stake, and configuration for wallets that operate a registered StargazeMPP provider.",
};

export default function ProviderOperatorPage() {
  return (
    <Placeholder
      eyebrow="Operator dashboard"
      title={
        <>
          Run your <em style={{ color: "var(--accent)" }}>provider</em>.
        </>
      }
      description="The operator surface for any wallet that's staked into the registry as a provider. Earnings, reputation snapshot, slashing events, recent crowd-vote outcomes, and metadata controls all in one place."
      roadmap={[
        "Revenue 30d + queries served + avg latency + refund rate stat cards.",
        "Reputation gauge (current score) + 12-week trend.",
        "Recent slashing events feed.",
        "Stake top-up + withdraw (subject to unbond delay).",
        "Pause / unpause provider, update endpoint URL, rotate metadata CID.",
      ]}
      cta={{ label: "Read provider onboarding docs", href: "/docs#providers" }}
    />
  );
}
