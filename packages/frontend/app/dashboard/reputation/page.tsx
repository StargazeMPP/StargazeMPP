import type { Metadata } from "next";
import { Placeholder } from "@/components/placeholder";

export const metadata: Metadata = {
  title: "Reputation",
  description:
    "Provider reputation scores across uptime, latency, and accuracy. Crowd-vote at 1 GAZE per vote.",
};

export default function ReputationPage() {
  return (
    <Placeholder
      eyebrow="Reputation oracle"
      title={
        <>
          Trust, <em style={{ color: "var(--accent)" }}>verifiable</em>.
        </>
      }
      description="Composite reputation per provider on a 0–1000 scale. Synthetic latency probes, crowd-verified accuracy votes, and AI-assisted quality assessment all feed in; oracle commits are signed on-chain."
      roadmap={[
        "Reputation gauge + 12-week sparkline per provider.",
        "Vote ACCURATE / INACCURATE with one click (1 GAZE burn).",
        "Recent slashing decisions + DAO vote tally.",
        "Filter by category and minimum reputation threshold.",
      ]}
      cta={{ label: "Open the dashboard", href: "/dashboard#reputation" }}
    />
  );
}
