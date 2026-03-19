import type { Address, Hex } from 'viem';

export const EIP712_VOUCHER_DOMAIN_NAME = 'StargazeMPP' as const;
export const EIP712_VOUCHER_DOMAIN_VERSION = '1' as const;

export interface VoucherDomain {
  name: typeof EIP712_VOUCHER_DOMAIN_NAME;
  version: typeof EIP712_VOUCHER_DOMAIN_VERSION;
  chainId: number;
  verifyingContract: Address;
}

export const VOUCHER_PRIMARY_TYPE = 'Voucher' as const;

export const VOUCHER_TYPES = {
  Voucher: [
    { name: 'sessionId', type: 'bytes32' },
    { name: 'agentWallet', type: 'address' },
    { name: 'provider', type: 'address' },
    { name: 'cumulativeAmount', type: 'uint256' },
    { name: 'nonce', type: 'uint256' },
    { name: 'expiry', type: 'uint64' },
  ],
} as const;

export interface VoucherMessage {
  sessionId: Hex;
  agentWallet: Address;
  provider: Address;
  cumulativeAmount: bigint;
  nonce: bigint;
  expiry: bigint;
}

export interface SignedVoucher {
  domain: VoucherDomain;
  message: VoucherMessage;
  signature: Hex;
}

export function buildVoucherDomain(chainId: number, escrowAddress: Address): VoucherDomain {
  return {
    name: EIP712_VOUCHER_DOMAIN_NAME,
    version: EIP712_VOUCHER_DOMAIN_VERSION,
    chainId,
    verifyingContract: escrowAddress,
  };
}
