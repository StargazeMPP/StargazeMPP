# `@stargazempp/shared`

Cross-package types, schemas, ABIs, and IDL — the typed boundary every other package imports from.

## What lives here

| Path | Contents |
|---|---|
| [`src/categories.ts`](src/categories.ts) | Canonical provider category enum: `on-chain-analytics`, `physical-ai`, `desci`, `rwa`, `compliance`, `ai-model`. |
| [`src/mpp/voucher.ts`](src/mpp/voucher.ts) | 133-byte Solana Ed25519 voucher schema (domain tag + canonical byte layout). |
| [`src/mpp/session.ts`](src/mpp/session.ts) | JWT session-token claims, in-memory session state, settlement threshold constants. |
| [`src/mpp/verifier.ts`](src/mpp/verifier.ts) | `MppVerifier` interface — voucher recovery + deposit verification. |
| [`src/mpp/payment-router.ts`](src/mpp/payment-router.ts) | `PaymentRouter` interface and routing-fee constants. |
| [`src/mpp/x402.ts`](src/mpp/x402.ts) | Solana-rail x402 USDC receipt shape. |
| [`src/vault/`](src/vault/) | Privacy-tier enum, Groth16 verifier-bundle shape, `VaultProofGenerator` interface. |
| [`src/solana/idl/`](src/solana/idl/) | Anchor IDL mirror for the four Solana programs. |
| [`src/solana/programs.ts`](src/solana/programs.ts) | Solana program IDs and canonical USDC mint constants. |
| [`src/db/`](src/db/) | Placeholder for the external-dev-owned Drizzle schema (currently empty). |

## Design rule

Types only. Zero runtime dependencies. Anything heavier belongs in a feature package.

## Build

```bash
npm run build
```
