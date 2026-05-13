# `@stargazempp/vault-circuits`

Groth16 circuits and Solana on-chain verifiers for the StargazeVault privacy
tiers.

## Tiers

| Tier | Circuit | Solana verifier | Use case |
|---|---|---|---|
| `zk-aggregate` | [`aggregate_sum`](circuits/aggregate_sum.circom) | `vault_verifier_aggregate_sum` | Cohort statistics — prove that the sum of N private inputs equals a public claimed value (e.g. health-cohort HRV totals). |
| `zk-aggregate` | `aggregate_mean` (planned) | not yet | Same shape as `aggregate_sum` for mean / variance proofs. |
| `confidential` | [`geofence`](circuits/geofence.circom) | `vault_verifier_geofence` | Prove a private (lat, lon) point lies within a public axis-aligned bounding box — OFAC / mission-corridor attestations à la Light Protocol. |
| `buyer-key` | `buyer_key_envelope` (planned) | `vault_verifier_buyer_key` (stub) | ERC-6551 token-bound-account wrap with per-buyer envelope encryption for raw drone / robot telemetry. Verifier currently rejects every proof until the circuit is finalised. |

## Build

```bash
npm install
npm run compile:aggregate   # aggregate_sum
npm run compile:geofence
```

Outputs to `build/`: `.r1cs` constraint system, `.wasm` witness generator,
`.sym` symbol map.

## Local dev setup

Dev artifacts use hard-coded entropy and are **not safe for mainnet**:

```bash
npm run all:aggregate-dev   # compile + dev Phase 2 for aggregate_sum
npm run all:geofence-dev    # same for geofence
```

These produce `build/<circuit>_final.zkey` and `build/<circuit>_vkey.json`.

## Emitting the Solana verifier vkey

The Solana verifier programs embed their vkey as a `pub const` byte array in
`programs/vault_verifier_<name>/src/vkey.rs`. Regenerate after every (re-)setup:

```bash
node scripts/emit-rust-vkey.mjs --circuit aggregate_sum --kind vkey \
  > ../anchor-program/programs/vault_verifier_aggregate_sum/src/vkey.rs

node scripts/emit-rust-vkey.mjs --circuit geofence --kind vkey \
  > ../anchor-program/programs/vault_verifier_geofence/src/vkey.rs
```

The script applies the BN254 big-endian / c1-first G2-limb transformation
required by Solana's `alt_bn128` syscalls. Same script's `--kind fixture`
mode emits `[u8; 256]` proof + `[[u8; 32]; N]` public-signal constants for
Rust integration tests.

## Trusted setup

Phase 2 ceremony per circuit, with at least five independent contributors
and a public randomness beacon for the final contribution. Final `.zkey`
artifacts are baked into the per-circuit Solana verifier programs and
registered against a provider via `stargaze_anchor::configure_vault`.

Detailed flow + attestation format: [`docs/vault-ceremony.md`](../../docs/vault-ceremony.md).
Deploy + per-provider registration: [`docs/vault-verifier-deployment.md`](../../docs/vault-verifier-deployment.md).

## Testing

```bash
npm test
```

Runs 9 vitest cases that exercise the off-chain proof generator. The
Solana verifier programs are tested separately under
`packages/anchor-program/tests/` (real litesvm proof rounds, dev-vkey
fixtures via `emit-rust-vkey.mjs --kind fixture`).
