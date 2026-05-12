import { describe, it, expect } from "vitest";
import {
  VOUCHER_PRIMARY_TYPE,
  VOUCHER_TYPES,
  EIP712_VOUCHER_DOMAIN_NAME,
  EIP712_VOUCHER_DOMAIN_VERSION,
  buildVoucherDomain,
} from "@stargazempp/shared";

describe("shared voucher schema", () => {
  it("uses the canonical primary type and domain identifiers", () => {
    expect(VOUCHER_PRIMARY_TYPE).toBe("Voucher");
    expect(EIP712_VOUCHER_DOMAIN_NAME).toBe("StargazeMPP");
    expect(EIP712_VOUCHER_DOMAIN_VERSION).toBe("1");
  });

  it("declares the same fields as the Solidity StargazeEscrow typehash", () => {
    expect(VOUCHER_TYPES.Voucher.map((f) => f.name)).toEqual([
      "sessionId",
      "agentWallet",
      "provider",
      "cumulativeAmount",
      "nonce",
      "expiry",
    ]);
  });

  it("builds an EIP-712 domain bound to the escrow contract + chain id", () => {
    const domain = buildVoucherDomain(31337, "0x1234567890123456789012345678901234567890");
    expect(domain.name).toBe(EIP712_VOUCHER_DOMAIN_NAME);
    expect(domain.version).toBe(EIP712_VOUCHER_DOMAIN_VERSION);
    expect(domain.chainId).toBe(31337);
    expect(domain.verifyingContract).toBe(
      "0x1234567890123456789012345678901234567890",
    );
  });
});
