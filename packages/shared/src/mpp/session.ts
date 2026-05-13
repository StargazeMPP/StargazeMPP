export type PaymentRail = 'solana';

export type PaymentMethod = 'solana' | 'stripe' | 'visa' | 'lightning';

export type SessionStatus = 'open' | 'settled' | 'expired';

/** Claims encoded into the JWT returned by `session.open` and carried on every subsequent request. */
export interface SessionTokenClaims {
  /** Agent wallet — base58 Solana pubkey. */
  agentWallet: string;
  /** Stable session identifier. */
  sessionId: string;
  /** Maximum spend across the whole session, as a decimal string of USDC base units (6 decimals). */
  spendingLimit: string;
  /** Unix seconds at which the session token expires regardless of remaining balance. */
  expiry: number;
  /** Which on-chain rail funded this session. */
  rail: PaymentRail;
}

/** A single voucher captured into the batch awaiting settlement. */
export interface SignedVoucherSnapshot {
  provider: string;
  cumulativeAmount: bigint;
  nonce: bigint;
  signature: string;
  /** Unix seconds when this voucher was accepted. */
  takenAt: number;
}

/** Authoritative session state — kept in Redis with 24h TTL. */
export interface SessionState {
  id: string;
  agentWallet: string;
  rail: PaymentRail;
  /** Total amount escrowed at `session.open`. */
  depositAmount: bigint;
  /** Agent-defined cap, ≤ depositAmount. */
  spendingLimit: bigint;
  /** Remaining unspent escrow. */
  balance: bigint;
  /** Highest cumulative amount seen so far, used to enforce strict monotonicity. */
  lastVoucherAmount: bigint;
  queryCount: number;
  /** Unix seconds. */
  openedAt: number;
  /** Unix seconds. */
  expiresAt: number;
  voucherBatch: SignedVoucherSnapshot[];
  status: SessionStatus;
}

/** Threshold at which the session manager force-settles even before the agent closes. */
export const SETTLEMENT_BALANCE_THRESHOLD_BPS = 1000; // 10 % of deposit

/** Default session TTL in seconds. */
export const SESSION_TTL_SECONDS = 24 * 60 * 60;
