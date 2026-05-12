import { describe, it, expect } from "vitest";
import {
  formatAmount,
  formatPriceTag,
  truncateAddress,
  explorerUrl,
} from "@/lib/format";

describe("formatAmount", () => {
  it("formats PathUSD (6 decimals)", () => {
    expect(formatAmount(8_000_000n, 6)).toBe("8");
    expect(formatAmount(1_500_000n, 6)).toBe("1.5");
    expect(formatAmount(1_234_500n, 6, 4)).toBe("1.2345");
  });

  it("renders thousands separators", () => {
    expect(formatAmount(1_234_567_000_000n, 6)).toBe("1,234,567");
  });

  it("drops trailing zeros in the fractional part", () => {
    expect(formatAmount(1_500_000n, 6, 6)).toBe("1.5");
  });

  it("handles GAZE-scale decimals", () => {
    expect(formatAmount(2_500_000_000_000_000_000n, 18)).toBe("2.5");
  });
});

describe("formatPriceTag", () => {
  it("includes currency and optional `per`", () => {
    expect(
      formatPriceTag({ amount: 8_000n, currency: "PathUSD", per: "query" }),
    ).toBe("0 PathUSD / query");
    expect(
      formatPriceTag({ amount: 8_000_000n, currency: "PathUSD", per: "query" }),
    ).toBe("8 PathUSD / query");
  });
});

describe("truncateAddress", () => {
  it("collapses long EVM addresses", () => {
    expect(
      truncateAddress("0x1234567890abcdef1234567890abcdef12345678"),
    ).toBe("0x1234…5678");
  });

  it("leaves short strings alone", () => {
    expect(truncateAddress("0xabcd")).toBe("0xabcd");
  });

  it("returns empty string for empty input", () => {
    expect(truncateAddress("")).toBe("");
  });
});

describe("explorerUrl", () => {
  it("links to TempoScan for the tempo rail", () => {
    expect(explorerUrl("0xabc", "tempo")).toContain("tempo.xyz");
  });
  it("links to Solscan for the solana rail", () => {
    expect(explorerUrl("FxYz", "solana")).toContain("solscan.io");
  });
});
