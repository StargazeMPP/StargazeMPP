# `@stargazempp/shared`

Cross-package types, schemas, ABIs, and IDL — the typed boundary every other package imports from.

## What lives here

| Path | Contents |
|---|---|
| [`src/categories.ts`](src/categories.ts) | Canonical provider category enum: `on-chain-analytics`, `physical-ai`, `desci`, `rwa`, `compliance`, `ai-model`. |
| [`src/mpp/voucher.ts`](src/mpp/voucher.ts) | EIP-712 domain, typed-data spec, and message shape for the cumulative MPP voucher. |
| [`src/mpp/session.ts`](src/mpp/session.ts) | JWT session-token claims, in-memory session state, settlement threshold constants. |
| [`src/mpp/verifier.ts`](src/mpp/verifier.ts) | `MppVerifier` interface — voucher recovery + deposit verification. |
| [`src/mpp/payment-router.ts`](src/mpp/payment-router.ts) | `PaymentRouter` interface and routing-fee constants. |
| [`src/mpp/x402.ts`](src/mpp/x402.ts) | Solana-rail x402 USDC receipt shape. |
| [`src/vault/`](src/vault/) | Privacy-tier enum, Groth16 verifier-bundle shape, `VaultProofGenerator` interface. |
| [`src/evm/abi/`](src/evm/abi/) | Auto-published ABIs for the five Tempo EVM contracts plus a manifest. |
| [`src/evm/addresses.ts`](src/evm/addresses.ts) | Per-network deployed contract address registry. |
| [`src/solana/idl/stargaze_anchor.json`](src/solana/idl/) | Anchor IDL for `StargazeAnchor`. |
| [`src/solana/programs.ts`](src/solana/programs.ts) | Solana program IDs and canonical USDC mint constants. |
| [`src/db/`](src/db/) | Drizzle schema source of truth shared between API gateway and indexer. |

## Design rule

Types only. No runtime dependencies beyond `viem` (peer) for `Address` / `Hex` types. Anything heavier belongs in a feature package.

## Build

```bash
npm run build
```
