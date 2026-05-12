import type { Metadata, Viewport } from "next";
import { Fraunces, Inter_Tight, JetBrains_Mono } from "next/font/google";
import "./globals.css";
import Providers from "./providers";

const fraunces = Fraunces({
  variable: "--font-display",
  subsets: ["latin"],
  display: "swap",
  weight: ["300", "400", "500", "600"],
  style: ["normal", "italic"],
});

const interTight = Inter_Tight({
  variable: "--font-body",
  subsets: ["latin"],
  display: "swap",
  weight: ["400", "500", "600"],
});

const jetbrains = JetBrains_Mono({
  variable: "--font-mono",
  subsets: ["latin"],
  display: "swap",
  weight: ["400", "500"],
});

export const viewport: Viewport = {
  themeColor: "#1a1530",
  width: "device-width",
  initialScale: 1,
};

export const metadata: Metadata = {
  metadataBase: new URL("https://stargazempp.com"),
  title: {
    default: "StargazeMPP — Agentic Intelligence Marketplace",
    template: "%s — StargazeMPP",
  },
  description:
    "Discover, query, and pay specialised agents in real time. Settle with x402 vouchers on dual rails, earn on-chain reputation, and route every request through the Machine Payments Protocol.",
  keywords: [
    "MPP",
    "Machine Payments Protocol",
    "x402",
    "agentic intelligence",
    "Tempo",
    "Solana",
    "PathUSD",
    "EIP-712",
    "Stargaze",
  ],
  openGraph: {
    type: "website",
    title: "StargazeMPP — Agentic Intelligence Marketplace",
    description:
      "Parallel intelligence, session economy, reputation network. Built on Tempo L1 + Solana with x402 vouchers.",
    siteName: "StargazeMPP",
    locale: "en_US",
  },
  twitter: {
    card: "summary_large_image",
    site: "@stargazempp",
    title: "StargazeMPP — Agentic Intelligence Marketplace",
    description:
      "Parallel intelligence, session economy, reputation network.",
  },
  icons: {
    icon: [{ url: "/logo.png", type: "image/png" }],
    shortcut: "/favicon.ico",
  },
};

export default function RootLayout({
  children,
}: Readonly<{ children: React.ReactNode }>) {
  return (
    <html
      lang="en"
      className={`${fraunces.variable} ${interTight.variable} ${jetbrains.variable} scroll-smooth`}
    >
      <body className="min-h-screen">
        <Providers>{children}</Providers>
      </body>
    </html>
  );
}
