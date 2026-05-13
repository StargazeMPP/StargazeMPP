# StargazeVault — verifier program deployment guide

Operational playbook for the three Groth16 verifier programs that sit behind
the `StargazeVault` privacy tiers, and for the per-provider `configure_vault`
registration that wires them into the marketplace. Pair this with
[`docs/vault-ceremony.md`](./vault-ceremony.md), which covers the trusted-setup
ceremony that produces the verifying keys these programs embed.

## At a glance

| Tier | Solana program | Cargo crate | Public signals | Status |
|---|---|---|---|---|
| `zk-aggregate` | `vault_verifier_aggregate_sum` | `programs/vault_verifier_aggregate_sum` | 1 (`claimedSum`) | dev vkey |
| `confidential` | `vault_verifier_geofence` | `programs/vault_verifier_geofence` | 4 (`minLat, maxLat, minLon, maxLon`) | dev vkey |
| `buyer-key` | `vault_verifier_buyer_key` | `programs/vault_verifier_buyer_key` | n/a | stub (always rejects) |

All three programs expose the same instruction —
`verify(proof_bytes: [u8; 256], public_signals: Vec<[u8; 32]>)` — and share a
single Anchor `global:verify` discriminator:

```text
0x85, 0xa1, 0x8d, 0x30, 0x78, 0xc6, 0x58, 0x96
```

`stargaze_anchor::submit_vault_proof` hard-codes this discriminator so it can
dispatch any per-circuit verifier through one manual CPI path.

## Program ids

Devnet / localnet keypairs (committed in
`packages/anchor-program/target/deploy/`) currently mint these program ids:

| Program | Program id (devnet) |
|---|---|
| `stargaze_anchor` | `m6P7kwvXoET9n5B8DFGwwLEozXdv6jBJPdbMiW1TH1R` |
| `vault_verifier_aggregate_sum` | `CTC7ehb1sYj7A5EsAd3E6viYdo5bxydzSpccDENbkUmP` |
| `vault_verifier_geofence` | `9d5rDusSqBH44dnJ4CQnR885xUoDq9NevgDHcsjYBaoD` |
| `vault_verifier_buyer_key` | `H2T3Amf7eTpeQQbzHkhTv5buCkfdc8bS41YwfgVkVsGn` |

Source of truth: `packages/anchor-program/Anchor.toml` and the `declare_id!`
macro in each program's `src/lib.rs`. After a redeploy that rotates any
program id, mirror the new id into:

1. `packages/anchor-program/Anchor.toml` — `[programs.localnet]` and
   `[programs.devnet]` (or new `[programs.mainnet]`) sections.
2. The `declare_id!` macro in the program's `src/lib.rs`.
3. `packages/shared/src/solana/programs.ts` (frontend / backend imports).
4. `packages/provider-sdk/src/internal/vault-proof.ts` —
   `STARGAZE_ANCHOR_PROGRAM_ID` is the only hardcoded id in the SDK; the
   verifier ids are passed in per-call by the caller.

A mismatch in any of these surfaces as `VerifierProgramMismatch` from
`submit_vault_proof`.

## Upgrade-authority recommendation

For each verifier program: **deploy once with a multisig authority for the
initial audit period, then mark the program immutable before mainnet
activation** (`solana program set-upgrade-authority --final`).

Rationale:

- Trail of Bits scope covers the deployed bytecode plus the embedded vkey.
  An upgrade-authority key in a hot wallet means ToB has to audit the
  upgrade path; an immutable program collapses that surface to a single
  bytecode + vkey artifact.
- Vkey rotation (post-ceremony, after a circuit change, etc.) requires a
  fresh deploy. We accept this cost in exchange for a closed-set audit
  boundary. The cost is one new program id per rotation — providers
  re-call `configure_vault` to point at the new id.
- `stargaze_anchor` itself is **not** immutable; its upgrade authority
  is the project multisig. Verifier programs are leaf dependencies and
  shouldn't share that authority.

Operator decision points (track in `BLOCKERS.md`):

- Per-verifier upgrade-authority key during the audit window — same
  multisig as `stargaze_anchor` or a separate per-verifier 2-of-3?
- Mainnet rent payer — protocol treasury or per-provider?
- Final commit gates the `--final` flip — ToB sign-off + ceremony
  attestation hash recorded in `docs/vault-ceremony.md`.

## Vkey provenance

Each verifier program carries its verifying key as `pub const VKEY:
Groth16Vkey` in `programs/<name>/src/vkey.rs`. The file is auto-generated;
the comment at the top of every `vkey.rs` records its source and ceremony
status.

The full chain of custody for a production vkey:

1. **Phase 1.** Reuse `powersOfTau28_hez_final_20.ptau` (Hermez universal
   ceremony). Pin its SHA-256 in `docs/vault-ceremony.md` before use.
2. **Phase 2.** Per-circuit ceremony with ≥5 independent contributors
   plus a randomness-beacon final contribution. Output: `<circuit>_final.zkey`.
3. **Export.** `snarkjs zkey export verificationkey <final>.zkey
   <circuit>_vkey.json`.
4. **Emit Rust.** `node packages/vault-circuits/scripts/emit-rust-vkey.mjs
   --circuit <name> --kind vkey > programs/vault_verifier_<name>/src/vkey.rs`.
   The script reads `build/<circuit>_vkey.json`, applies the BN254 big-endian
   encoding (c1-first G2 limbs to match Solana's `alt_bn128` syscalls), and
   writes a deterministic Rust file.
5. **Audit gate.** Re-run the per-program litesvm tests (the fixture's
   public signals are still valid; the proof and vkey bytes change). ToB
   reviews the diff to `vkey.rs` against the published ceremony attestation.
6. **Arweave bundle.** Upload `<circuit>_final.zkey`, `<circuit>_vkey.json`,
   and the generated `vkey.rs` to Arweave; the CID is the value providers
   pass as `arweave_cid` to `configure_vault`.
7. **Deploy.** `anchor build` + `solana program deploy` against the
   per-program keypair (see "Build + deploy" below).
8. **Attest.** Append the ceremony hash, the program id, the deployed
   bytecode hash, and the Arweave CID to
   `docs/vault-ceremony-attestations/<circuit>.md`.

## Dev vkeys today — caveat lector

All three verifier programs currently ship with **dev vkeys** generated by
`scripts/setup-dev.mjs`, which runs Phase 1 + Phase 2 against a hard-coded
entropy string. They are intentionally insecure: a single party knows the
toxic waste. Every `vkey.rs` carries a `// DEV VKEY — DO NOT DEPLOY TO
MAINNET` banner. Re-bake before any mainnet program is deployed.

The `buyer_key` program is a deeper kind of placeholder: it has no circuit
at all and rejects every proof with `CircuitNotFinalised`. Providers can
still call `configure_vault` with the `BuyerKey` tier and the stub program
id — the on-chain registry accepts it — but `submit_vault_proof` will fail
until a real circuit ships. See "Buyer-key circuit" below.

## Build + deploy

Everything is one Anchor workspace. From `packages/anchor-program/`:

```bash
# Build every program (stargaze_anchor + 3 verifiers + tests).
anchor build

# Run the full test suite (94 litesvm tests across 16 files; loads all
# 4 programs into one SVM via setup_svm_with_verifiers).
cargo test -p stargaze_anchor_tests --tests --no-fail-fast

# Deploy one verifier (devnet). Each program has its own keypair under
# target/deploy/<name>-keypair.json; the address there must match the
# Anchor.toml entry and the program's declare_id!.
solana program deploy \
    --url https://api.devnet.solana.com \
    --program-id target/deploy/vault_verifier_aggregate_sum-keypair.json \
    target/deploy/vault_verifier_aggregate_sum.so

# Repeat for geofence and buyer_key.
solana program deploy \
    --url https://api.devnet.solana.com \
    --program-id target/deploy/vault_verifier_geofence-keypair.json \
    target/deploy/vault_verifier_geofence.so

solana program deploy \
    --url https://api.devnet.solana.com \
    --program-id target/deploy/vault_verifier_buyer_key-keypair.json \
    target/deploy/vault_verifier_buyer_key.so

# After deploy: refresh the IDL mirror so the off-chain SDKs see the
# latest discriminators and account shapes.
cp target/idl/*.json ../shared/src/solana/idl/
```

For mainnet, swap `--url` to `https://api.mainnet-beta.solana.com` (or your
preferred RPC) and confirm the rent payer is the intended treasury wallet
before submitting. Each verifier `.so` is ~250 KB so the rent + deploy fee
is in the low single-digit SOL range per program.

## Compute-budget notes

Per-ix CU expectations, indicative (`cargo test -- --nocapture` and the
upcoming `BENCH.md` carry measured numbers):

- `aggregate_sum.verify` — pairing + 1 scalar mul → ~250 k CU.
- `geofence.verify` — pairing + 4 scalar muls → ~400 k CU.
- `stargaze_anchor.submit_vault_proof` wrapper (sha256 commitment check,
  PDA init, manual CPI overhead) → ~30 k CU on top of the verifier.

All comfortably below the 1.4 M default. The provider-sdk helper still
recommends attaching a `ComputeBudgetInstruction::set_compute_unit_limit(600_000)`
to give the verifier headroom and to absorb future vkey size changes.

## Per-provider registration

The on-chain registry side is unchanged from the original vault design —
the verifier's program id is just a `Pubkey` field on `VaultConfig`.

```rust
stargaze_anchor::configure_vault(
    ctx,
    provider_id,          // [u8; 32]
    VaultTier::ZkAggregate,
    aggregate_sum_program_id,  // Pubkey
    arweave_cid,          // [u8; 32] — points at the proof bundle
)
```

The instruction is permissioned: the signer must equal
`Provider.owner`. Re-configuring (e.g. when rotating from an old vkey
program id to a new one) overwrites `tier`, `on_chain_verifier`,
`arweave_cid`, and `active`; it preserves `auditor_key` and
`buyer_key_rotation_cid` so those rotations stay independent.

Setting `tier = Open` means `submit_vault_proof` is not callable for this
provider — the handler short-circuits with `TierDoesNotRequireProof`. Use
the `Open` tier for providers that don't need cryptographic privacy.

`Pubkey::default()` is treated as "no verifier set"; `submit_vault_proof`
rejects with `VerifierUnset`. This lets a provider be registered with a
non-`Open` tier while the verifier program is still under audit.

## Agent-side: submitting a proof

The provider-sdk exposes everything needed to build a `submit_vault_proof`
transaction without depending on `@coral-xyz/anchor`:

```ts
import {
  buildSubmitVaultProofInstruction,
  computeVaultProofSignalsHash,
  deriveVaultConfigPda,
  deriveVaultProofRecordPda,
} from '@stargazempp/provider-sdk/internal/vault-proof';
import { ComputeBudgetProgram, PublicKey, Transaction } from '@solana/web3.js';

// 1. Run snarkjs.groth16.fullProve off-chain, then transform the proof
//    for Solana: negate proof.pi_a.y, reorder G2 limbs c1-first.
//    See packages/vault-circuits/scripts/emit-rust-vkey.mjs for the exact
//    transformation (the `emitFixture` helper does the same encoding
//    used here).
const proofBytes: Uint8Array = solanaEncodedProof; // 256 bytes
const publicSignals: Uint8Array[] = solanaEncodedSignals; // each 32 bytes

// 2. Build the ix. The helper derives both PDAs and the signals_hash so
//    callers don't need to know about seeds or borsh framing.
const { instruction, proofRecordPda } = buildSubmitVaultProofInstruction({
  submitter: walletPubkey,
  verifierProgramId: new PublicKey('CTC7ehb1sYj7A5EsAd3E6viYdo5bxydzSpccDENbkUmP'),
  providerId: providerIdBytes32,
  proofBytes,
  publicSignals,
});

// 3. Wrap with a compute-budget bump and send.
const tx = new Transaction().add(
  ComputeBudgetProgram.setComputeUnitLimit({ units: 600_000 }),
  instruction,
);
await connection.sendTransaction(tx, [wallet]);
```

The submitter does **not** have to be `provider.owner`; anyone with a
funded wallet can post a proof on a provider's behalf. The
`VaultProofRecord` PDA seeds are `[b"vault_proof", provider_id,
signals_hash]` and the account is `init`-only — collision means the
proof has already been submitted and the second tx reverts with
`AccountAlreadyInUse`. This is the replay guard; do not work around it
by deriving a fresh `signals_hash` for the same public inputs.

## Failure modes

Mapping for the new errors `submit_vault_proof` can surface:

| Error code | Cause | Remediation |
|---|---|---|
| `VaultInactive` | Provider's vault was deactivated by the admin. | Provider re-runs `configure_vault`. |
| `VerifierUnset` | `VaultConfig.on_chain_verifier == Pubkey::default()`. | Provider calls `configure_vault` with a real verifier id. |
| `VerifierProgramMismatch` | The `verifier_program` account passed to the ix does not equal `VaultConfig.on_chain_verifier`. | Caller passes the right program id (the SDK reads it from `VaultConfig` if you pull from on-chain). |
| `ProofVerificationFailed` | Verifier program CPI rejected the proof (bad pairing, wrong signal count, malformed encoding). | Re-run the snarkjs preprocessing; confirm the vkey on chain matches the `.zkey` you proved against. |
| `TierDoesNotRequireProof` | Vault tier is `Open`. | Don't submit; this tier doesn't run circuits. |
| `SignalsHashMismatch` | The 32-byte `signals_hash` argument is not `sha256` of the concatenated `public_signals`. | Use `computeVaultProofSignalsHash` (provider-sdk) or `hashv(...)` on the same byte representation. |
| `AccountAlreadyInUse` (system program) | A `VaultProofRecord` PDA for `(provider_id, signals_hash)` already exists — replay. | Change the public signals (i.e. prove a different statement) or skip the resubmit. |

## Vkey rotation playbook

For a vkey rotation that does **not** change public-signal counts (e.g. a
re-ceremony for the same circuit):

1. Run the new Phase 2 ceremony per `docs/vault-ceremony.md`. Publish
   attestations.
2. Regenerate `vkey.rs` via `emit-rust-vkey.mjs`. Commit alone — easy diff
   review.
3. Decide whether to **rotate the program id** (recommended for ToB
   clarity — old id stays auditable, new id carries the new vkey) or to
   **upgrade in place** (only possible if the program is still under a
   non-`--final` upgrade authority).
4. Build, deploy, refresh the IDL mirror, update the `declare_id!` if the
   id rotated.
5. Bundle the new vkey + zkey + vkey.rs to Arweave; record CID.
6. Each provider on the rotated tier calls `configure_vault` again with
   the new program id and new `arweave_cid`. Until they do, their
   existing `VaultProofRecord` PDAs remain valid history but new proofs
   must target the new program.
7. Append the rotation entry to
   `docs/vault-ceremony-attestations/<circuit>.md`.

A vkey rotation that **changes the public-signal count** (e.g. switching
`aggregate_sum` from `N=8` to `N=16`) is a different circuit and gets a
different program (`vault_verifier_aggregate_sum_n16`). Don't try to share
program ids across signal-count variants — the on-chain wrapper has the
count baked in via `Vec::try_into::<[[u8;32]; N]>`.

## Buyer-key circuit

`vault_verifier_buyer_key` is intentionally a no-op stub. The circuit
specification (per-buyer envelope encryption keyed off a token-bound
account, à la ERC-6551) is not finalised, so the program returns
`CircuitNotFinalised` for every input. This lets the registry surface
stay stable while we design the real circuit:

- Providers can `configure_vault` with `tier = BuyerKey` and the stub
  program id today.
- `submit_vault_proof` will fail until the real verifier ships.
- When the circuit lands, the rollout is a normal "deploy new program +
  re-`configure_vault`" rotation (see the playbook above).

Open design points the operator owes us before circuit work begins:

- The precise statement being proved (token-bound account ownership +
  envelope-encryption preimage match? something else?).
- Whether per-buyer keys live on Solana, on Arweave, or in a Light
  Protocol-style off-chain index.
- The public-signal count.

## Reference

- Anchor program: `packages/anchor-program/programs/stargaze_anchor/src/lib.rs`
  (section header `VAULT PROOF: instructions`, around line 1022).
- Shared verifier crate: `packages/anchor-program/crates/vault-verifier-core/`.
- Verifier programs: `packages/anchor-program/programs/vault_verifier_*/`.
- Vkey emit script: `packages/vault-circuits/scripts/emit-rust-vkey.mjs`.
- Dev setup script: `packages/vault-circuits/scripts/setup-dev.mjs`.
- Provider-sdk helper: `packages/provider-sdk/src/internal/vault-proof.ts`.
- Trusted-setup ceremony plan: [`docs/vault-ceremony.md`](./vault-ceremony.md).
