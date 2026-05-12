# Internal brief — this team's working list

Mirror of `EXTERNAL_DEV_BRIEF.md` but for what we own. Keep light; the briefs serve as the seam, not a Gantt chart.

## Week 1 deliverables (so external dev isn't blocked)

1. `packages/contracts-evm` — Foundry init, source files for `GAZEToken`, `BurnController`, `StargazeEscrow`, `StargazeRegistry`, `PrivacyVaultRegistry`. Stub implementations are fine; ABIs need to be stable enough for `packages/shared/evm/abi/` to publish.
2. `packages/shared` scaffolding — publish:
   - `evm/abi/*.json` (even placeholder)
   - `evm/addresses.ts` with `{ tempoTestnet: {...} }` once we deploy stubs to testnet
   - `mpp/voucher.ts` — EIP-712 typed data spec
   - `mpp/session.ts` — JWT claims interface
   - `mpp/verifier.ts` — `MppVerifier` interface that the backend imports
   - `mpp/payment-router.ts` — `PaymentRouter.settle(sessionId, vouchers)` interface
   - `vault/verifier-bundle.ts` — Groth16 bundle shape (stub: empty union of tiers)
   - `vault/proof-generator.ts` — `VaultProofGenerator` interface
3. `packages/anchor-program` — Anchor init, empty `StargazeAnchor` program with the cross-chain registry mirror instructions stubbed.
4. `packages/indexer` — Rust crate init, Yellowstone gRPC client wired, but emits to stdout only for now. Schema-aware projection comes once `packages/shared/db/schema.ts` lands.
5. `packages/vault-circuits` — circom scaffold for ZK-AGGREGATE. Trusted-setup ceremony plan in `docs/vault-ceremony.md`.

## Subsequent work

Track in TaskCreate per-session. Major lines:

- Tempo testnet deployments → record addresses in `packages/shared/evm/addresses.ts`.
- Solana devnet deployment of `StargazeAnchor` → IDL into `packages/shared/solana/idl/`.
- CCIP plumbing across Tempo + Solana + Base for `$GAZE` and provider-registry mirroring.
- Groth16 trusted setup (Phase 2 ceremony per circuit).
- Trail of Bits engagement (contracts + circuits).
- Certora spec for `GAZEToken` transfer hook.
- Immunefi bounty draft (contracts + circuit perimeters).

## Constraints to keep top-of-mind

- 4-of-7 Safe multisig + 14-day timelock on every Tempo EVM upgrade.
- Sub-10ms voucher validation budget — `MppVerifier` cannot do an RPC call.
- ZK proof generation can be expensive; queue and stream where possible, document any > 1s p95 paths.
- No PII in any contract event or program log.
- $9.14B agentic-commerce TAM in 2026 is the macro; we're racing the ecosystem, not just shipping a product.
