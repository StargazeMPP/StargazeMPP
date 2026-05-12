/**
 * Convert a `bigint` amount of the smallest unit into a decimal string,
 * rounded to a presentable precision. PathUSD and USDC both have 6
 * decimals; $GAZE has 18.
 */
export function formatAmount(amount: bigint, decimals: number, maxFractionDigits = 2): string {
  const base = 10n ** BigInt(decimals);
  const whole = amount / base;
  const frac = amount % base;
  if (frac === 0n || maxFractionDigits === 0) {
    return whole.toLocaleString("en-US");
  }
  const fracStr = frac.toString().padStart(decimals, "0").slice(0, maxFractionDigits).replace(/0+$/, "");
  if (fracStr.length === 0) return whole.toLocaleString("en-US");
  return `${whole.toLocaleString("en-US")}.${fracStr}`;
}

export interface PriceTagValue {
  amount: bigint;
  currency: "PathUSD" | "USDC" | "GAZE";
  per?: string;
}

const DECIMALS: Record<PriceTagValue["currency"], number> = {
  PathUSD: 6,
  USDC: 6,
  GAZE: 18,
};

export function formatPriceTag(value: PriceTagValue): string {
  const formatted = formatAmount(value.amount, DECIMALS[value.currency]);
  const tail = value.per ? ` / ${value.per}` : "";
  return `${formatted} ${value.currency}${tail}`;
}
