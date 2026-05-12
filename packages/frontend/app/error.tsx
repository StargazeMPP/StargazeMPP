"use client";

import { useEffect } from "react";
import Link from "next/link";

interface Props {
  error: Error & { digest?: string };
  reset: () => void;
}

export default function GlobalError({ error, reset }: Props) {
  useEffect(() => {
    console.error("[stargaze:error]", error);
  }, [error]);

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
          — Unexpected error
        </p>
        <h1
          className="font-light text-4xl leading-tight"
          style={{ fontFamily: "var(--font-display)" }}
        >
          The page didn’t load.
        </h1>
        <p className="mt-4 text-sm" style={{ color: "var(--ink-dim)" }}>
          Something broke on our end. The error has been logged. You can try again, or head home.
        </p>
        {error.digest && (
          <p
            className="mt-3 text-[11px]"
            style={{ fontFamily: "var(--font-mono)", color: "var(--ink-muted)" }}
          >
            ref · {error.digest}
          </p>
        )}
        <div className="mt-8 flex items-center justify-center gap-3">
          <button
            onClick={reset}
            className="px-5 py-2.5 rounded-full text-sm"
            style={{ background: "var(--accent)", color: "var(--bg)" }}
          >
            Try again
          </button>
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
