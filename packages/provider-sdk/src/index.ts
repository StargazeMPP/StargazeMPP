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
  VoucherDomain,
  VoucherMessage,
  X402Receipt,
} from '@stargazempp/shared';

export {
  EIP712_VOUCHER_DOMAIN_NAME,
  EIP712_VOUCHER_DOMAIN_VERSION,
  VOUCHER_PRIMARY_TYPE,
  VOUCHER_TYPES,
  buildVoucherDomain,
} from '@stargazempp/shared';
