import fs from "node:fs";
import path from "node:path";
import LandingHydrate from "./_landing-hydrate";

export const revalidate = 60;

function readLandingMarkup(): string {
  const file = path.join(process.cwd(), "app", "_landing.html");
  const raw = fs.readFileSync(file, "utf-8");
  // Strip the <html>/<head>/<body> wrappers so the marketing markup can
  // slot inside the Next.js root layout.
  return raw
    .replace(/^[\s\S]*?<head[^>]*>/i, "")
    .replace(/<\/head>[\s\S]*?<body[^>]*>/i, "")
    .replace(/<\/body>[\s\S]*$/i, "");
}

export default function MarketingLandingPage() {
  const inner = readLandingMarkup();
  return <LandingHydrate html={inner} />;
}
