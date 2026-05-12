import { recoverTypedDataAddress } from 'viem';
import type { SignedVoucher, VerifiedVoucher } from '@stargazempp/shared';
import { VOUCHER_TYPES, VOUCHER_PRIMARY_TYPE } from '@stargazempp/shared';

/**
 * Recover the agent wallet from an EIP-712 signed voucher. Pure crypto —
 * no RPC, no I/O. The hot-path call inside the session manager's
 * `validateVoucher`.
 *
 * Does **not** enforce session-level invariants (monotonicity, spending
 * limit, expiry); those live in the session manager. This function only
 * answers "who signed this, and what's the claimed amount?".
 */
export async function recoverVoucherSigner(voucher: SignedVoucher): Promise<VerifiedVoucher> {
  const agentWallet = await recoverTypedDataAddress({
    domain: voucher.domain,
    types: VOUCHER_TYPES,
    primaryType: VOUCHER_PRIMARY_TYPE,
    message: voucher.message,
    signature: voucher.signature,
  });

  return {
    sessionId: voucher.message.sessionId,
    agentWallet,
    provider: voucher.message.provider,
    cumulativeAmount: voucher.message.cumulativeAmount,
    nonce: voucher.message.nonce,
  };
}
