"use client";

import { useSignTypedData } from "wagmi";
import type { Hex, Address } from "viem";
import {
  VOUCHER_TYPES,
  VOUCHER_PRIMARY_TYPE,
  buildVoucherDomain,
  type SignedVoucher,
  type VoucherMessage,
} from "@stargazempp/shared";
import { env } from "../env";

export interface SignVoucherInput {
  sessionId: Hex;
  agentWallet: Address;
  provider: Address;
  cumulativeAmount: bigint;
  nonce: bigint;
  expiry: bigint;
}

/**
 * Hook that returns a function for signing an EIP-712 voucher against
 * `StargazeEscrow`. Uses the canonical typed-data schema from
 * `@stargazempp/shared` — keep the field set in lockstep with the
 * Solidity `VOUCHER_TYPEHASH`.
 *
 * The escrow address and chain id are pulled from `lib/env.ts`.
 */
export function useVoucherSigner(): (input: SignVoucherInput) => Promise<SignedVoucher> {
  const { signTypedDataAsync } = useSignTypedData();
  const domain = buildVoucherDomain(env.tempo.chainId, env.contracts.sessionEscrow);

  return async (input) => {
    const message: VoucherMessage = {
      sessionId: input.sessionId,
      agentWallet: input.agentWallet,
      provider: input.provider,
      cumulativeAmount: input.cumulativeAmount,
      nonce: input.nonce,
      expiry: input.expiry,
    };
    const signature = await signTypedDataAsync({
      domain,
      types: VOUCHER_TYPES,
      primaryType: VOUCHER_PRIMARY_TYPE,
      message,
    });
    return { domain, message, signature };
  };
}
