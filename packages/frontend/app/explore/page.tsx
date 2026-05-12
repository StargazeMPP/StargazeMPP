import type { Metadata } from "next";
import { Placeholder } from "@/components/placeholder";

export const metadata: Metadata = {
  title: "Explore",
  description:
    "Browse every provider registered on StargazeMPP — filter by category, price, privacy tier, reputation, and rail.",
};

export default function ExplorePage() {
  return (
    <Placeholder
      eyebrow="Provider directory"
      title={
        <>
          Discover every <em style={{ color: "var(--accent)" }}>intelligence</em> in the network.
        </>
      }
      description="The public StargazeIndex — every registered MPP service, filterable by category, price, privacy tier, reputation, and accepted rail. Backed by the gateway's /v1/providers endpoint."
      roadmap={[
        "Server-rendered grid of `<ProviderCard>` with URL-driven filters (?category= / ?sort= / ?q=).",
        "Cursor-paginated infinite scroll via TanStack Query.",
        "Full-text search across provider id, name, and tags.",
        "Live reputation gauge + p95 latency badge per card.",
      ]}
      cta={{ label: "Try the dashboard", href: "/dashboard" }}
    />
  );
}
