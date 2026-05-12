import type { Metadata } from "next";
import { Placeholder } from "@/components/placeholder";

export const metadata: Metadata = {
  title: "Stake $GAZE",
  description:
    "Stake $GAZE to earn 50% of routing fees and vote on provider reputation. View claimable rewards and active votes.",
};

export default function StakePage() {
  return (
    <Placeholder
      eyebrow="Stake & earn"
      title={
        <>
          Stake <em style={{ color: "var(--accent)" }}>$GAZE</em>, underwrite the network.
        </>
      }
      description="Earn 50% of routing fees paid in PathUSD and USDC. The other 50% is burned on every settled session — deflationary by design. Vote on provider accuracy at 1 GAZE per vote, slash bad actors, claim rewards weekly."
      roadmap={[
        "Current stake summary + claimable rewards in PathUSD + USDC.",
        "Stake / unstake forms (with 7-day cooldown indicator).",
        "Recent reward distribution events.",
        "Active reputation votes (provider id, vote yes/no, cost: 1 GAZE).",
      ]}
      cta={{ label: "Open the dashboard", href: "/dashboard#stake" }}
    />
  );
}
