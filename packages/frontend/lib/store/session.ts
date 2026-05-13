"use client";

import { create } from "zustand";
import { persist, createJSONStorage } from "zustand/middleware";
import type { PaymentRail } from "@stargazempp/shared";

export interface ActiveSession {
  sessionId: `0x${string}` | string;
  rail: PaymentRail;
  escrow: bigint;
  spendingLimit: bigint;
}

interface SessionStoreState {
  sessionId?: string;
  rail?: PaymentRail;
  escrow?: bigint;
  spendingLimit?: bigint;
  cumulative: bigint;
  nonce: bigint;
  queryCount: number;

  open: (session: ActiveSession) => void;
  recordQuery: (cost: bigint) => void;
  close: () => void;
  reset: () => void;
}

const INITIAL = {
  sessionId: undefined,
  rail: undefined,
  escrow: undefined,
  spendingLimit: undefined,
  cumulative: 0n,
  nonce: 0n,
  queryCount: 0,
} satisfies Omit<SessionStoreState, "open" | "recordQuery" | "close" | "reset">;

/**
 * Session state for the StargazeMPP playground / dashboard. Tracks the
 * active sessionId, rail, escrow balance, and a `spendingLimit` cap that
 * matches the Solana voucher schema.
 *
 * Persisted to `sessionStorage` so a refresh keeps the active session,
 * but a new tab starts fresh — sessions are short-lived by design.
 *
 * Bigints survive serialisation thanks to the JSON storage replacer.
 */
export const useSessionStore = create<SessionStoreState>()(
  persist(
    (set) => ({
      ...INITIAL,
      open: (session) =>
        set({
          sessionId: session.sessionId,
          rail: session.rail,
          escrow: session.escrow,
          spendingLimit: session.spendingLimit,
          cumulative: 0n,
          nonce: 0n,
          queryCount: 0,
        }),
      recordQuery: (cost) =>
        set((s) => ({
          cumulative: s.cumulative + cost,
          nonce: s.nonce + 1n,
          queryCount: s.queryCount + 1,
        })),
      close: () => set(INITIAL),
      reset: () => set(INITIAL),
    }),
    {
      name: "stargaze:session",
      storage: createJSONStorage(() => sessionStorage, {
        replacer: (_key, value) =>
          typeof value === "bigint" ? { __bigint: value.toString() } : value,
        reviver: (_key, value) => {
          if (value && typeof value === "object" && "__bigint" in value) {
            return BigInt((value as { __bigint: string }).__bigint);
          }
          return value;
        },
      }),
    },
  ),
);
