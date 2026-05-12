import { createFileRoute } from "@tanstack/react-router";
import { useEffect, useRef } from "react";
import siteHtml from "../site.html?raw";

export const Route = createFileRoute("/")({
  component: Index,
});

function Index() {
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!ref.current) return;
    // Re-execute inline scripts that dangerouslySetInnerHTML doesn't run.
    const scripts = Array.from(ref.current.querySelectorAll("script"));
    for (const old of scripts) {
      const s = document.createElement("script");
      for (const attr of Array.from(old.attributes)) {
        s.setAttribute(attr.name, attr.value);
      }
      s.text = old.textContent ?? "";
      old.replaceWith(s);
    }
  }, []);

  // Strip out wrapping <html>/<head>/<body> so we can inject the body content
  // (including <style>, <link>, and <script> tags) into the page.
  const inner = siteHtml
    .replace(/^[\s\S]*?<head[^>]*>/i, "")
    .replace(/<\/head>[\s\S]*?<body[^>]*>/i, "")
    .replace(/<\/body>[\s\S]*$/i, "");

  return <div ref={ref} dangerouslySetInnerHTML={{ __html: inner }} />;
}
