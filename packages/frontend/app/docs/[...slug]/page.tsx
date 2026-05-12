import type { Metadata } from "next";
import { Placeholder } from "@/components/placeholder";

interface Params {
  slug?: string[];
}

export async function generateMetadata({
  params,
}: {
  params: Promise<Params>;
}): Promise<Metadata> {
  const { slug } = await params;
  const path = slug?.join(" / ") ?? "Docs";
  return {
    title: `Docs · ${path}`,
    description: `StargazeMPP developer documentation — ${path}.`,
  };
}

export default async function DocsCatchallPage({
  params,
}: {
  params: Promise<Params>;
}) {
  const { slug } = await params;
  const heading = slug?.join(" / ") ?? "Docs";
  return (
    <Placeholder
      eyebrow={`Docs · ${heading}`}
      title={
        <>
          Deep <em style={{ color: "var(--accent)" }}>documentation</em>, soon.
        </>
      }
      description={`MDX-driven developer documentation. The single-page guide at /docs is the launch surface; per-topic pages (architecture, x402, sessions, rails, SDK quickstart) get their own URLs once the MDX system is wired through next-mdx-remote.`}
      roadmap={[
        "File-system routing from `content/docs/**/*.mdx`.",
        "shiki-highlighted code blocks with copy buttons.",
        "Sidebar navigation + on-page TOC.",
        "Versioning + 'Edit on GitHub' link per page.",
      ]}
      cta={{ label: "Read the single-page docs", href: "/docs" }}
    />
  );
}
