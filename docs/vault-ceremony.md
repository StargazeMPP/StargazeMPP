# StargazeVault — trusted-setup ceremony plan

Each Groth16 circuit registered with the StargazeVault per-provider registry
requires a per-circuit Phase 2 trusted setup. The output `.zkey` plus the
corresponding on-chain Solana verifier program is what providers stake against;
a compromised setup means provider stakes can be silently rugged. This doc
tracks the ceremony per circuit and is the canonical attestation index.

Pair with [`docs/vault-verifier-deployment.md`](./vault-verifier-deployment.md),
which covers the deploy + register + rotate flow once a ceremony output exists.

## Phase 1 (universal)

Reuse the [Hermez / Polygon Phase 1](https://github.com/iden3/snarkjs#7-prepare-phase-2)
ptau (up to 2^20 constraints). Do **not** run our own Phase 1 — that would
require coordinating dozens of independent contributors and is wasteful when
an audited universal ceremony already exists.

- Download: `powersOfTau28_hez_final_20.ptau` (~640 MB).
- Verify hash before use: `snarkjs powersoftau verify <ptau>`.
- Pin the SHA-256 in this doc (TBD — operator to fill in before the first
  production ceremony) so every contributor can verify their input matches
  the published ptau.

## Phase 2 (per circuit)

### Circuits in scope

| Circuit | Tier | Cohort size N | Public signals | Status |
|---|---|---|---|---|
| `aggregate_sum` | `zk-aggregate` | 8 | 1 (`claimedSum`) | dev vkey; Phase 2 pending |
| `aggregate_mean` | `zk-aggregate` | TBD | TBD | not yet written |
| `geofence` | `confidential` | n/a | 4 (`minLat, maxLat, minLon, maxLon`) | dev vkey; Phase 2 pending |
| `buyer_key_envelope` | `buyer-key` | n/a | TBD | circuit not yet written; verifier ships as always-rejecting stub |

### Per-circuit ceremony flow

For each circuit `<name>`:

1. **Compile** — `npm run compile:<name>` (writes
   `build/<name>.r1cs`, `build/<name>_js/<name>.wasm`).
2. **Phase 2 init** — `snarkjs groth16 setup build/<name>.r1cs
   powersOfTau28_hez_final_20.ptau build/<name>_0000.zkey`.
3. **Contributor rotation** — at least **5** independent contributors,
   serial (not parallel — each contributor must verify the previous
   attestation's hash before contributing). Each:
   - Receives the latest `.zkey` over an authenticated channel (Signal
     preferred).
   - Runs `snarkjs zkey contribute <prev>.zkey <next>.zkey
     --name="<handle>" -v -e="<entropy>"`.
   - Publishes the contribution hash (`snarkjs zkey verify` output) on a
     public Git commit in this repo under
     `docs/vault-ceremony-attestations/<circuit>.md`.
   - Securely destroys their toxic-waste entropy source (RAMdisk only —
     never written to persistent media; video evidence per contributor;
     hashes recorded in the attestation file).
4. **Beacon** — final contribution uses a public randomness beacon (NIST
   Beacon pulse preferred over BTC block hash — more legible; document
   the round-index decision rule: first pulse ≥ T+24h after the final
   human contribution). Run via `snarkjs zkey beacon`.
5. **Finalisation** — `snarkjs zkey verify <final>.zkey` plus
   `snarkjs zkey export verificationkey <final>.zkey <name>_vkey.json`.
6. **Verifier emit** — Solana-target only:
   `node packages/vault-circuits/scripts/emit-rust-vkey.mjs --circuit
   <name> --kind vkey > packages/anchor-program/programs/vault_verifier_<name>/src/vkey.rs`.
   The script applies the BN254 BE / c1-first-G2 encoding required by
   Solana's `alt_bn128` syscalls.
7. **Audit gate** — Trail of Bits reviews the per-program `vkey.rs` diff
   against the published attestations, alongside the verifier program
   `lib.rs` and the shared `vault-verifier-core` crate.
8. **Arweave bundle** — upload `<name>_final.zkey`, `<name>_vkey.json`,
   and the generated `vkey.rs` to Arweave; record the CID in the
   attestation file. The same CID is what providers pass as
   `arweave_cid` to `configure_vault`.
9. **On-chain registration** — `anchor build`,
   `solana program deploy target/deploy/vault_verifier_<name>.so` against
   the per-program keypair, then providers call
   `stargaze_anchor::configure_vault(provider_id, tier, verifier_program_id,
   arweave_cid)`. See
   [`docs/vault-verifier-deployment.md`](./vault-verifier-deployment.md)
   for the deploy + register flow in detail.

### Contributors

To be appointed before M2 start. Minimum **5**, at least **3 not employed by
the StargazeMPP team**. Candidate pool will be cross-listed across the
launch partner projects (AirborneLabs, AxonMed, Synapse, TrigonFi) so that
no single team can collude to compromise a circuit. Names and GitHub
handles enumerated in `docs/vault-ceremony-attestations/<circuit>.md` once
appointed.

### Anti-correlation

Contributions happen in **serial**, not parallel. Each contributor must
verify the previous attestation's hash on-chain (or in this repo) before
contributing. Each contributor's entropy source is independent (different
machines, different random sources, RAMdisk-only).

## Attestation format

For each circuit, `docs/vault-ceremony-attestations/<circuit>.md` carries:

- Phase 1 ptau hash (SHA-256) and mirror URL used.
- Per-contributor: handle, GitHub identity, contribution hash from
  `snarkjs zkey verify`, RAMdisk teardown video URL, signature of the
  contribution hash by the contributor's PGP / wallet key.
- Beacon source + pulse / block id + finalisation hash.
- Final vkey JSON hash (SHA-256).
- Generated `vkey.rs` hash (SHA-256) and the commit that landed it.
- Arweave bundle CID.
- Deployed program id + bytecode hash + slot of deploy tx.
- ToB review sign-off reference.

## Open issues

- Pin specific `ptau` version + provenance hash here once chosen.
- Decide beacon source (NIST vs BTC). NIST is more legible; BTC is
  cheaper to audit later. Recommend NIST.
- Coordinate audit timing — ToB review of verifier programs is on the
  M3 path.
- Confirm whether `aggregate_mean` is in scope for v1 or deferred — it
  is currently scaffolded with a Solidity verifier in `build/` but no
  Solana verifier program.
- Specify the `buyer_key_envelope` circuit. The verifier currently ships
  as `vault_verifier_buyer_key`, an always-rejecting stub
  (`CircuitNotFinalised`). See the "Buyer-key circuit" section in
  [`docs/vault-verifier-deployment.md`](./vault-verifier-deployment.md).
