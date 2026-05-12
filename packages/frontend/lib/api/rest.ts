import { env } from "../env";
import { ApiError } from "./errors";

export interface ExecuteQueryInput {
  providerId: string;
  body: unknown;
  sessionId: `0x${string}` | string;
  cumulative: bigint;
  voucherSig: `0x${string}`;
}

/**
 * Latency-critical voucher path. Goes browser → gateway directly
 * (skipping the Next.js API layer) so the round-trip stays under the
 * 100 ms target from the frontend build spec §8.2.
 */
export async function executeQuery(input: ExecuteQueryInput): Promise<unknown> {
  const res = await fetch(`${env.apiUrl}/v1/query/${input.providerId}`, {
    method: "POST",
    headers: {
      "content-type": "application/json",
      "x-mpp-session": String(input.sessionId),
      "x-mpp-cumulative": input.cumulative.toString(),
      "x-mpp-voucher": input.voucherSig,
    },
    body: JSON.stringify(input.body),
  });
  if (!res.ok) throw await ApiError.fromResponse(res);
  return res.json();
}

export interface OpenSessionInput {
  rail: "tempo" | "solana";
  depositTxHash: string;
  spendingLimit: string;
}

export async function openSession(input: OpenSessionInput): Promise<{ sessionId: string; token: string }> {
  const res = await fetch(`${env.apiUrl}/v1/sessions`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(input),
  });
  if (!res.ok) throw await ApiError.fromResponse(res);
  return res.json();
}

export async function closeSession(sessionId: string, finalVoucherSig: `0x${string}`): Promise<{ settlementTx: string }> {
  const res = await fetch(`${env.apiUrl}/v1/sessions/${sessionId}/close`, {
    method: "POST",
    headers: {
      "content-type": "application/json",
      "x-mpp-voucher": finalVoucherSig,
    },
  });
  if (!res.ok) throw await ApiError.fromResponse(res);
  return res.json();
}
