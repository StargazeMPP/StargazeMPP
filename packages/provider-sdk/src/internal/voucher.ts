import nacl from 'tweetnacl';
import type { SignedVoucher, VerifiedVoucher } from '@stargazempp/shared';
import { VOUCHER_DOMAIN_TAG, VOUCHER_MESSAGE_LEN } from '@stargazempp/shared';

/**
 * Recover and validate the agent who signed a Solana voucher. Pure crypto —
 * no RPC. The hot-path call inside the session manager's `validateVoucher`.
 *
 * Steps:
 *   1. Length-check the message (133 bytes) and signature (64 bytes).
 *   2. Verify the 21-byte domain prefix.
 *   3. Verify the Ed25519 signature with tweetnacl.
 *   4. Decode the fixed-offset fields into `VerifiedVoucher`.
 *
 * Throws on any failure. Does NOT enforce session-level invariants
 * (monotonicity, spending limit, expiry) — those live in the session manager.
 */
export async function recoverVoucherSigner(voucher: SignedVoucher): Promise<VerifiedVoucher> {
  if (voucher.message.length !== VOUCHER_MESSAGE_LEN) {
    throw new Error(
      `recoverVoucherSigner: message must be ${VOUCHER_MESSAGE_LEN} bytes, got ${voucher.message.length}`,
    );
  }
  if (voucher.signature.length !== 64) {
    throw new Error(
      `recoverVoucherSigner: signature must be 64 bytes, got ${voucher.signature.length}`,
    );
  }
  if (voucher.agentWallet.length !== 32) {
    throw new Error(
      `recoverVoucherSigner: agentWallet must be 32 bytes, got ${voucher.agentWallet.length}`,
    );
  }

  // Domain prefix.
  for (let i = 0; i < VOUCHER_DOMAIN_TAG.length; i++) {
    if (voucher.message[i] !== VOUCHER_DOMAIN_TAG[i]) {
      throw new Error('recoverVoucherSigner: voucher domain prefix mismatch');
    }
  }

  const ok = nacl.sign.detached.verify(voucher.message, voucher.signature, voucher.agentWallet);
  if (!ok) {
    throw new Error('recoverVoucherSigner: Ed25519 signature verification failed');
  }

  // Layout: domain(0..21) | sessionId(21..53) | agentWallet(53..85) |
  //         providerId(85..117) | cumulativeAmount u64-le(117..125) |
  //         nonce u64-le(125..133)
  const msg = voucher.message;
  const sessionId = msg.slice(21, 53);
  const agentWallet = msg.slice(53, 85);
  const providerId = msg.slice(85, 117);

  const view = new DataView(msg.buffer, msg.byteOffset, msg.byteLength);
  const cumLo = view.getUint32(117, true);
  const cumHi = view.getUint32(121, true);
  const cumulativeAmount = BigInt(cumLo) | (BigInt(cumHi) << 32n);
  const nonceLo = view.getUint32(125, true);
  const nonceHi = view.getUint32(129, true);
  const nonce = BigInt(nonceLo) | (BigInt(nonceHi) << 32n);

  return {
    sessionId,
    agentWallet,
    providerId,
    cumulativeAmount,
    nonce,
  };
}
