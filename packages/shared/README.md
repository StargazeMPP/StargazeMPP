# `packages/shared` — types, schemas, ABIs

**Owner:** shared.

The seam between this team and the external dev. Anything that crosses the boundary lives here:

- `evm/abi/` — generated ABIs for `GAZEToken`, `BurnController`, `StargazeEscrow`, `StargazeRegistry`, `PrivacyVaultRegistry`.
- `solana/idl/` — Anchor IDL for `StargazeAnchor`.
- `mpp/` — EIP-712 voucher schema, session token JWT claims, x402 receipt shape.
- `vault/` — Groth16 verifying-key bundle shape, public-output schemas per privacy tier.
- `db/` — Drizzle schema source of truth shared by backend + indexer.
- `categories.ts` — `on-chain-analytics | physical-ai | desci | rwa | compliance | ai-model`.

Rule: types only, no runtime deps. If you find yourself adding a heavy dependency to `shared`, it belongs in a feature package instead.
