import Link from "next/link";

export default function NotFound() {
  return (
    <div
      className="min-h-screen flex items-center justify-center px-6 text-[var(--ink)]"
      style={{ background: "var(--bg)", fontFamily: "var(--font-body)" }}
    >
      <div className="max-w-md text-center">
        <p
          className="text-[11px] uppercase tracking-[0.16em] mb-3"
          style={{ fontFamily: "var(--font-mono)", color: "var(--accent)" }}
        >
          — Lost in space
        </p>
        <h1
          className="font-light text-7xl leading-none"
          style={{ fontFamily: "var(--font-display)" }}
        >
          404
        </h1>
        <p
          className="mt-4 text-2xl"
          style={{ fontFamily: "var(--font-display)" }}
        >
          That page isn’t in the index.
        </p>
        <p className="mt-3 text-sm" style={{ color: "var(--ink-dim)" }}>
          The URL may have changed, or the provider may have been deregistered.
        </p>
        <div className="mt-8 flex items-center justify-center gap-3">
          <Link
            href="/explore"
            className="px-5 py-2.5 rounded-full text-sm"
            style={{ background: "var(--accent)", color: "var(--bg)" }}
          >
            Browse providers
          </Link>
          <Link
            href="/"
            className="px-5 py-2.5 rounded-full text-sm border"
            style={{ borderColor: "var(--line-2)", color: "var(--ink)" }}
          >
            Go home
          </Link>
        </div>
      </div>
    </div>
  );
}
