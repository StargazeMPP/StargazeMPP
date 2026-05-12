import Link from "next/link";
import type { ReactNode } from "react";

interface Props {
  eyebrow: string;
  title: ReactNode;
  description: string;
  roadmap?: string[];
  cta?: { label: string; href: string };
}

/**
 * Brand-consistent placeholder shell. Used for routes scoped in the
 * frontend build spec but not yet implemented — keeps SEO + design
 * cohesive while the feature is shipped behind it.
 */
export function Placeholder({ eyebrow, title, description, roadmap, cta }: Props) {
  return (
    <div
      className="min-h-screen text-[var(--ink)]"
      style={{ background: "var(--bg)", fontFamily: "var(--font-body)" }}
    >
      <header
        className="sticky top-0 z-30 backdrop-blur-md border-b"
        style={{ borderColor: "var(--line)", background: "rgba(26,21,48,0.72)" }}
      >
        <div className="max-w-[1400px] mx-auto px-5 sm:px-8 lg:px-12 h-16 flex items-center justify-between">
          <Link href="/" className="flex items-center gap-2.5">
            <img src="/logo.png" alt="" className="w-8 h-8 object-contain" />
            <span style={{ fontFamily: "var(--font-display)" }} className="text-[17px]">
              StargazeMPP
            </span>
          </Link>
          <nav className="flex items-center gap-6 text-sm">
            <Link href="/docs" className="text-[var(--ink-muted)] hover:text-[var(--ink)]">
              Docs
            </Link>
            <Link
              href="/dashboard"
              className="px-4 py-2 rounded-full"
              style={{ background: "var(--accent)", color: "var(--bg)" }}
            >
              Launch app
            </Link>
          </nav>
        </div>
      </header>

      <main className="max-w-3xl mx-auto px-5 sm:px-8 py-24">
        <p
          className="text-[11px] uppercase tracking-[0.16em] text-[var(--accent)] mb-4"
          style={{ fontFamily: "var(--font-mono)" }}
        >
          — {eyebrow}
        </p>
        <h1
          className="font-light text-5xl lg:text-6xl leading-[1.05]"
          style={{ fontFamily: "var(--font-display)", letterSpacing: "-0.02em" }}
        >
          {title}
        </h1>
        <p
          className="mt-6 text-[15px] leading-[1.75]"
          style={{ color: "var(--ink-dim)" }}
        >
          {description}
        </p>

        {roadmap && roadmap.length > 0 && (
          <div className="mt-10">
            <p
              className="text-[11px] uppercase tracking-[0.16em] mb-4"
              style={{ fontFamily: "var(--font-mono)", color: "var(--ink-muted)" }}
            >
              On the roadmap
            </p>
            <ul className="space-y-3">
              {roadmap.map((item) => (
                <li
                  key={item}
                  className="rounded-xl border px-5 py-4 text-[14px] leading-relaxed"
                  style={{
                    borderColor: "var(--line)",
                    background:
                      "linear-gradient(180deg, rgba(196,179,236,0.06), rgba(196,179,236,0.02))",
                    color: "var(--ink-dim)",
                  }}
                >
                  {item}
                </li>
              ))}
            </ul>
          </div>
        )}

        {cta && (
          <Link
            href={cta.href}
            className="inline-flex mt-10 items-center gap-2 px-5 py-3 rounded-full text-sm"
            style={{ background: "var(--accent)", color: "var(--bg)" }}
          >
            {cta.label} →
          </Link>
        )}
      </main>
    </div>
  );
}
