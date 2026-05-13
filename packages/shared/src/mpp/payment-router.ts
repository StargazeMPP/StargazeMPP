import type { SignedVoucherSnapshot } from './session.js';

export interface SettlementResult {
  /** Tempo (or Solana) tx hash of the batch settlement. */
  txHash: string;
  /** Sum of all voucher amounts paid to providers. */
  totalSettled: bigint;
  /** 2 % of `totalSettled`, taken from the escrow before payout. */
  routingFee: bigint;
  /** Unused escrow returned to the agent wallet. */
  refundToAgent: bigint;
}

/**
 * Bridges the session manager's in-memory voucher batch to the on-chain
 * `StargazeEscrow.settle(...)` call. Implemented by this team.
 */
export interface PaymentRouter {
  settle(sessionId: string, vouchers: SignedVoucherSnapshot[]): Promise<SettlementResult>;
}

/** Routing fee in basis points — taken from each settled session. */
export const ROUTING_FEE_BPS = 200; // 2 %
