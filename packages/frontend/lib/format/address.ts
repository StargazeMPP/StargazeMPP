/**
 * Render an EVM `0x…` or Solana base58 address with a short prefix and
 * suffix. Default leaves 6 leading and 4 trailing characters; both
 * conventions match popular block explorers.
 */
export function truncateAddress(address: string, leading = 6, trailing = 4): string {
  if (!address) return "";
  if (address.length <= leading + trailing + 1) return address;
  return `${address.slice(0, leading)}…${address.slice(-trailing)}`;
}

export function explorerUrl(address: string, chain: "tempo" | "solana"): string {
  if (chain === "tempo") return `https://tempo.xyz/explorer/address/${address}`;
  return `https://solscan.io/account/${address}`;
}
