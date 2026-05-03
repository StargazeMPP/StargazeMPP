# StargazeVault — trusted-setup ceremony plan

Each Groth16 circuit registered with `PrivacyVaultRegistry` requires a per-circuit Phase 2 trusted setup. The output `.zkey` plus the corresponding on-chain verifier is what providers stake against; a compromised setup means provider stakes can be silently rugged. This doc tracks the ceremony per circuit.

## Phase 1 (universal)

Reuse the [Hermez / Polygon Phase 1](https://github.com/iden3/snarkjs#7-prepare-phase-2) ptau (up to 2^20 constraints). Do **not** run our own Phase 1 — that would require coordinating dozens of independent contributors and is wasteful when an audited universal ceremony already exists.

- Download: `powersOfTau28_hez_final_20.ptau` (~640 MB).
- Verify hash before use: `snarkjs powersoftau verify <ptau>`.

## Phase 2 (per circuit)

### Circuits in scope

| Circuit | Tier | Cohort size N | Status |
|---|---|---|---|
| `aggregate_sum` | `zk-aggregate` | 8 | scaffolded — Phase 2 pending |
| `aggregate_mean` | `zk-aggregate` | TBD | not yet written |
| `geofence` | `confidential` (Kalder pattern) | n/a | not yet written |
| `buyer_key_envelope` | `buyer-key` | n/a | not yet written |

### Per-circuit ceremony flow

For each circuit `<name>`:

1. **Compile** — `bun run compile:<name>` (writes `build/<name>.r1cs`, `build/<name>_js/<name>.wasm`).
2. **Phase 2 init** — `snarkjs groth16 setup build/<name>.r1cs powersOfTau28_hez_final_20.ptau build/<name>_0000.zkey`.
3. **Contributor rotation** — at least **5** independent contributors, each:
   - Receives the latest `.zkey` over an authenticated channel (Signal preferred).
   - Runs `snarkjs zkey contribute <prev>.zkey <next>.zkey --name="Alice" -v -e="<entropy>"`.
   - Publishes the contribution hash (`snarkjs zkey verify` output) on a public Git commit in this repo under `docs/vault-ceremony-attestations/<circuit>.md`.
   - Securely destroys their toxic-waste entropy source (RAMDISK ideally).
4. **Beacon** — final contribution uses a public randomness beacon (NIST Beacon or recent BTC block hash; document which) via `snarkjs zkey beacon`.
5. **Finalisation** — `snarkjs zkey verify <final>.zkey` + `snarkjs zkey export verificationkey`.
6. **Verifier emit** — `snarkjs zkey export solidityverifier build/<name>_final.zkey build/<NameCamel>Verifier.sol`.
7. **Audit gate** — Trail of Bits reviews the verifier contract + circuit alongside the rest of the EVM contracts.
8. **On-chain registration** — deploy the verifier; call `PrivacyVaultRegistry.configure(providerId, tier, verifier, arweaveCid)`.

### Contributors

To be appointed before M2 start. Minimum 5, at least 3 not employed by the StargazeMPP team. Candidate pool will be cross-listed across the partner projects (AirborneLabs, AxonMed, Synapse, TrigonFi) so that no single team can collude to compromise a circuit.

### Anti-correlation

Contributions happen in serial, NOT parallel. Each contributor must verify the previous attestation's hash on-chain (or in this repo) before contributing.

## Open issues

- Pin specific `ptau` version + provenance hash in this doc once chosen.
- Decide beacon source (BTC vs NIST). NIST is more legible; BTC is cheaper to audit later.
- Coordinate audit timing — Trail of Bits review of verifiers is in M3 per the overview-PDF roadmap.
