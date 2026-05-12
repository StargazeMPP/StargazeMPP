"use client";

import { useEffect, useRef } from "react";

/**
 * Renders pre-stripped marketing HTML and re-executes any inline `<script>`
 * tags that `dangerouslySetInnerHTML` skips. The marketing page is authored
 * as plain HTML (`app/_landing.html`) so designers can iterate without React
 * tooling; this component is the only bridge.
 */
export default function LandingHydrate({ html }: { html: string }) {
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const root = ref.current;
    if (!root) return;
    const scripts = Array.from(root.querySelectorAll("script"));
    for (const old of scripts) {
      const next = document.createElement("script");
      for (const attr of Array.from(old.attributes)) {
        next.setAttribute(attr.name, attr.value);
      }
      next.text = old.textContent ?? "";
      old.replaceWith(next);
    }
  }, [html]);

  return <div ref={ref} dangerouslySetInnerHTML={{ __html: html }} />;
}
