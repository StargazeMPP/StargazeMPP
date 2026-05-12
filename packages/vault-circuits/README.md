# `@stargazempp/vault-circuits`

Groth16 circuits and on-chain verifiers for the StargazeVault privacy tiers.

## Tiers

| Tier | Circuit | Use case |
|---|---|---|
| `zk-aggregate` | [`aggregate_sum`](circuits/aggregate_sum.circom) | Cohort statistics — prove that the sum of N private inputs equals a public claimed value (e.g. health-cohort HRV totals). |
| `zk-aggregate` | `aggregate_mean` (planned) | Same shape as `aggregate_sum` for mean / variance proofs. |
| `confidential` | `geofence` (planned) | OFAC / mission-corridor attestations à la Light Protocol. |
| `buyer-key` | `buyer_key_envelope` (planned) | ERC-6551 token-bound-account wrap with per-buyer envelope encryption for raw drone / robot telemetry. |

## Build

```bash
npm install
npm run compile:aggregate
```

Outputs to `build/`: `.r1cs` constraint system, `.wasm` witness generator, `.sym` symbol map.

## Trusted setup

Phase 2 ceremony per circuit, with at least five independent contributors and a public randomness beacon for the final contribution. Final `.zkey` files and the corresponding Solidity verifiers register with [`PrivacyVaultRegistry`](../contracts-evm/src/PrivacyVaultRegistry.sol).

For local development:

```bash
npm run setup:aggregate-dev   # Dev-only Phase 2 with hard-coded entropy.
npm run verifier:aggregate    # Emit AggregateSumVerifier.sol.
```

Dev artifacts are not safe for mainnet.
