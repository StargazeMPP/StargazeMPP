// Public surface.
//
// The external dev fleshes out the ergonomic decorator API on top of the
// internal verifier exported below. The crypto primitive (voucher
// recovery, deposit verification) lives in `./internal/`.

export { StargazeMppVerifier, type StargazeMppVerifierOptions } from './internal/verifier.js';
export { recoverVoucherSigner } from './internal/voucher.js';
export { parseX402Receipt, type X402ReceiptParserOptions } from './internal/x402-receipt.js';

// Convenience re-exports so consumers only need one import.
export type {
  DepositProof,
  MppVerifier,
  SignedVoucher,
  VerifiedDeposit,
  VerifiedVoucher,
  VoucherMessage,
  X402Receipt,
} from '@stargazempp/shared';

export {
  VOUCHER_DOMAIN_TAG,
  VOUCHER_MESSAGE_LEN,
  buildVoucherMessage,
} from '@stargazempp/shared';
