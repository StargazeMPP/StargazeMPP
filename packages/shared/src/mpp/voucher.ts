/**
 * Solana voucher schema used by the StargazeMPP escrow program (Anchor
 * `settle` instruction). The off-chain agent signs `buildVoucherMessage(...)`
 * with its Ed25519 keypair (`session.agentWallet`), and the resulting
 * (signature, pubkey, message) triple is fed into the Ed25519 native
 * precompile that precedes every `settle` call.
 *
 * Schema (133 bytes exactly):
 *   offset 0..21  : domain tag `b"StargazeMPP/Voucher/1"` (21 ASCII bytes)
 *   offset 21..53 : sessionId  (32 bytes)
 *   offset 53..85 : agentWallet pubkey (32 bytes)
 *   offset 85..117: providerId (32 bytes)
 *   offset 117..125: cumulativeAmount (u64, little-endian)
 *   offset 125..133: nonce (u64, little-endian)
 */

/** ASCII bytes of `StargazeMPP/Voucher/1` — must match the on-chain `VOUCHER_DOMAIN_TAG`. */
export const VOUCHER_DOMAIN_TAG: Uint8Array = new Uint8Array(
  // "StargazeMPP/Voucher/1"
  [
    0x53, 0x74, 0x61, 0x72, 0x67, 0x61, 0x7a, 0x65, 0x4d, 0x50, 0x50, 0x2f, 0x56, 0x6f,
    0x75, 0x63, 0x68, 0x65, 0x72, 0x2f, 0x31,
  ],
);

/** Exact byte length of the voucher message that the agent signs. */
export const VOUCHER_MESSAGE_LEN = 133 as const;

export interface VoucherMessage {
  /** 32-byte session identifier (raw bytes). */
  sessionId: Uint8Array;
  /** 32-byte agent wallet pubkey (raw bytes). */
  agentWallet: Uint8Array;
  /** 32-byte provider identifier (raw bytes). */
  providerId: Uint8Array;
  /** Cumulative amount the agent authorises to spend, in USDC base units (6 decimals). */
  cumulativeAmount: bigint;
  /** Monotonic nonce per (session, provider). */
  nonce: bigint;
}

/** Pack a u64 into 8 little-endian bytes. */
function u64LeBytes(value: bigint): Uint8Array {
  const out = new Uint8Array(8);
  let v = value;
  for (let i = 0; i < 8; i++) {
    out[i] = Number(v & 0xffn);
    v >>= 8n;
  }
  return out;
}

function assertLen(name: string, buf: Uint8Array, expected: number): void {
  if (buf.length !== expected) {
    throw new Error(`${name} must be ${expected} bytes, got ${buf.length}`);
  }
}

/**
 * Build the canonical 133-byte voucher message that the agent must sign with
 * its Ed25519 keypair. Returns a freshly-allocated `Uint8Array`.
 */
export function buildVoucherMessage(args: VoucherMessage): Uint8Array {
  assertLen('sessionId', args.sessionId, 32);
  assertLen('agentWallet', args.agentWallet, 32);
  assertLen('providerId', args.providerId, 32);

  const out = new Uint8Array(VOUCHER_MESSAGE_LEN);
  out.set(VOUCHER_DOMAIN_TAG, 0);
  out.set(args.sessionId, 21);
  out.set(args.agentWallet, 53);
  out.set(args.providerId, 85);
  out.set(u64LeBytes(args.cumulativeAmount), 117);
  out.set(u64LeBytes(args.nonce), 125);
  return out;
}
