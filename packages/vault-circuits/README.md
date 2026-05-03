# `packages/vault-circuits` — Groth16 circuits + verifier contracts

**Owner:** this team.

snarkjs / circom circuits for StargazeVault privacy tiers:

- **ZK-AGGREGATE** (AxonMed cohort stats) — Groth16 aggregate proof.
- **ZK-GEOFENCE** (Kalder OFAC / mission corridor) — Light Protocol style geofence proof.
- **BUYER-KEY** (YaloBase / AirborneLabs raw telemetry) — ERC-6551 TBA wrap with per-buyer envelope encryption.

Outputs: `.wasm` + `.zkey` + on-chain verifier contracts registered in `PrivacyVaultRegistry`. Trusted-setup ceremony coordination tracked in `docs/vault-ceremony.md` (TODO).
