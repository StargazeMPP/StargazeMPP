import type { Metadata } from "next";
import { Placeholder } from "@/components/placeholder";

interface Params {
  id: string;
}

export async function generateMetadata({
  params,
}: {
  params: Promise<Params>;
}): Promise<Metadata> {
  const { id } = await params;
  return {
    title: `Provider · ${id}`,
    description: `Pricing, reputation, latency, request schema, and sample queries for the ${id} provider on StargazeMPP.`,
  };
}

export default async function ProviderDetailPage({
  params,
}: {
  params: Promise<Params>;
}) {
  const { id } = await params;
  return (
    <Placeholder
      eyebrow={`Provider · ${id}`}
      title={
        <>
          Full <em style={{ color: "var(--accent)" }}>provider</em> detail.
        </>
      }
      description={`Everything an agent or evaluator needs to integrate ${id}: pricing card, health (uptime + p50/p95 latency + error rate), request/response schema viewer, copy-paste cURL / TS / Python snippets, and reputation history.`}
      roadmap={[
        "Header card with logo, category badge, verified checkmark, reputation gauge, stake amount.",
        "Health sparkline of 24h latency (Recharts).",
        "JSON-schema-derived request + response panels (collapsible tree).",
        "Reputation history line chart for the last 12 weeks.",
        "Deep link to `/playground?provider=" + id + "` for instant testing.",
      ]}
      cta={{ label: "Test in playground", href: `/playground?provider=${id}` }}
    />
  );
}
