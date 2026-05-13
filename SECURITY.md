# Security model and invariants

Scope: the on-chain surface that holds value or gates trust in StargazeMPP.
Out of scope: off-chain services (backend, indexer projections, frontend);
those interact with the on-chain state read-only.

In scope:

- `packages/anchor-program/programs/stargaze_anchor` — monolithic Anchor
  program: registry, reputation, staking ($GAZE), escrow / voucher
  settlement, vault registry, vault-proof dispatch, x402 receipts.
- `packages/anchor-program/programs/vault_verifier_aggregate_sum` — Groth16
  verifier (1 public signal).
- `packages/anchor-program/programs/vault_verifier_geofence` — Groth16
  verifier (4 public signals).
- `packages/anchor-program/programs/vault_verifier_buyer_key` — currently a
  stub: rejects every input with `CircuitNotFinalised`.
- `packages/anchor-program/crates/vault-verifier-core` — shared `no_std`
  wrapper over `groth16-solana`'s `alt_bn128` syscalls.
- The manual Ed25519 precompile verifier embedded in `stargaze_anchor::lib`
  (module `ed25519_verify`).
- The manual CPI dispatch from `stargaze_anchor::submit_vault_proof` into
  the verifier programs.

The deployed program ids are recorded in
`packages/anchor-program/Anchor.toml`. The verifying keys baked into the
verifier programs are **dev vkeys** today; mainnet requires a real
trusted-setup ceremony per
[`docs/vault-ceremony.md`](./docs/vault-ceremony.md).

## Trust model

| Principal | Role | How it's authenticated |
|---|---|---|
| `Config.authority` | Protocol admin: init_escrow, set_stake_mint, init_staking, deactivate_vault, slash, process_routing_fee_burn, set_reputation_score | Signer in `Initialize` selects it; rotates only via redeploy |
| `Provider.owner` | Per-provider owner: configure_vault, set_vault_auditor_key, set_vault_buyer_key_rotation_cid | Signer of `register_provider`; PDA `[b"provider", provider_id]` |
| `UsdcConfig.router` | Off-chain Payment Router: settle, close_session pre-expiry | Pubkey field set on `init_escrow`; rotates only via redeploy |
| Stake owner | Owner of a `[b"stake", provider_id, owner]` PDA | Signer in `stake`, `request_unstake`, `claim_unstake` |
| Agent wallet | Counterparty to a `Session`: open_session, signs vouchers, self-closes post-expiry | Ed25519 keypair; only the on-chain pubkey is tracked |
| Vault proof submitter | Anyone with a funded wallet — submission is permissionless | Just needs to pay rent for the `VaultProofRecord` PDA |

Failure modes outside this trust model that the on-chain code **does
not** defend against:

- A compromised `Config.authority` can call `slash` on any staker (capped
  at `stake.amount - stake.cooldown_amount`) and `set_reputation_score`
  on any provider. Mitigation is operational (the authority is a project
  multisig), not on-chain.
- A compromised `UsdcConfig.router` can settle any voucher it has a valid
  signature for, and can close any session before expiry. Vouchers are
  Ed25519-signed by the agent so the router cannot forge spend, but it
  can choose **when** to submit and which voucher in a session's chain
  to stop at.
- A compromised verifier-program upgrade authority can swap the embedded
  vkey for one that accepts arbitrary proofs. Recommendation in
  [`docs/vault-verifier-deployment.md`](./docs/vault-verifier-deployment.md)
  is to mark each verifier program immutable before mainnet activation.

## Invariants

### Escrow / voucher settlement

E1. **Accounting identity.** For any session `s` over its lifetime:
    `s.deposit == s.total_spent + s.total_fee + refund_to_agent`. Verified
    by `escrow.rs::settle_then_close_preserves_accounting_identity` over a
    range of deposits. `s.total_spent` and `s.total_fee` are monotonic and
    only mutated by `settle`; the refund is computed at `close_session`
    time from the residual vault balance.

E2. **Voucher replay guard.** Two settles with the same canonical voucher
    message bytes fail; the second cannot succeed because the
    `[b"voucher", session_id, sha256(message)]` PDA was `init`'d during
    the first call and the system program rejects re-init. Verified by
    `escrow.rs::settle_replay_rejected`.

E3. **Voucher cumulative monotonicity (per provider).** Each
    `(session_id, provider_id)` pair maintains a `VoucherCursor` PDA
    that stores `last_cumulative`. Every settle asserts
    `cumulative_amount > last_cumulative`. Equal or lower values revert
    with `NonMonotonic`. Verified by `escrow.rs::settle_non_monotonic_rejected`.

E4. **Voucher signature integrity.** The `settle` handler asserts:
    (a) the preceding instruction in the same tx is the Ed25519 precompile;
    (b) the precompile's pubkey matches `session.agent_wallet`;
    (c) the precompile's message bytes equal `build_voucher_message_bytes(...)`
    for the ix arguments;
    (d) `sha256(message)` equals the `message_hash` argument used as the
    PDA seed.
    Missing precompile → `MissingPrecompile`. Mismatched signer → `WrongSigner`.
    Mismatched bytes → `WrongMessage`. Verified by
    `escrow.rs::settle_rejects_{missing_precompile_ix, bad_signer, wrong_message_bytes}`.

E5. **Voucher domain separation.** The voucher message begins with the
    fixed tag `b"StargazeMPP/Voucher/1"` and has a fixed 133-byte length.
    A signed message from any other protocol with a different tag or
    length cannot replay as a StargazeMPP voucher.

E6. **Spending limit cap.** Every settle asserts
    `cumulative_amount <= session.spending_limit`. The limit is in turn
    constrained at `open_session` time to `<= session.deposit`, so the
    cap is always reachable from the deposit. Verified by
    `escrow.rs::settle_spending_limit_cap` and
    `escrow.rs::open_session_rejects_limit_above_deposit`.

E7. **Routing fee rate.** `ROUTING_FEE_BPS = 200` (2%). Computed per
    settle as `mul_bps(delta, 200)` where `delta = cumulative_amount -
    last_cumulative`. Checked-arithmetic prevents wrap-around;
    `NumericalOverflow` on overflow.

E8. **Session settled idempotency.** `settle` and `close_session` both
    refuse to operate on a session whose `settled == true`. Verified by
    `escrow.rs::settle_after_close_rejected`.

E9. **Session expiry.** `settle` rejects if `now >= session.expires_at`
    (`SessionExpired`). `close_session` before expiry is gated to either
    the router (`UsdcConfig.router`) or the agent (`session.agent_wallet`)
    only — random signers revert; the agent revert before expiry surfaces
    as `SessionNotExpired`. Verified by
    `escrow.rs::{settle_after_expiry_rejected, close_session_unauthorized_before_expiry}`.

E10. **Router gating on settle.** Only `UsdcConfig.router` may invoke
    `settle`. A signer mismatch reverts with `UnauthorizedRouter`. Verified
    by `escrow.rs::settle_rejects_non_router`.

E11. **PDA-derived vault authority.** Token transfers out of the
    `session_vault` are signed by the
    `[b"session_vault", session_id]` PDA. Routing-fee transfers credit
    `[b"routing_fee_vault"]`. Both authorities are program-derived; no
    external key can drain them.

### Staking + slash + burn ladder

S1. **Stake amount accounting.** `StakeAccount.amount` only increases via
    `stake` and only decreases via `claim_unstake` or `slash`. Each path
    uses `checked_add` / `checked_sub` and reverts on overflow.

S2. **Cooldown enforcement.** `claim_unstake` requires
    `now - cooldown_start_ts >= cooldown_secs`; pre-cooldown claims revert
    with `CooldownActive`. Each `request_unstake` resets `cooldown_start_ts`
    to the current slot — back-to-back requests extend the cooldown rather
    than running independent clocks. Verified by the `staking.rs` suite (7
    tests).

S3. **Slash cap.** `slash` transfers `min(amount, stake.amount -
    stake.cooldown_amount)`, never exceeding the slashable portion of the
    stake. Cooldown-queued amount is **not** slashable. Slash output is
    routed to the constant `BURN_DESTINATION` ATA; the handler asserts
    `burn_destination_ata.owner == BURN_DESTINATION` so the admin cannot
    redirect a slash payout to a wallet they control.

S4. **Stake-mint binding.** `set_stake_mint` (admin) sets the SPL mint
    that all subsequent `stake` / `request_unstake` / `claim_unstake` /
    `slash` / `cast_reputation_vote` / `reputation_vote_burn` operations
    are constrained to. Each call asserts the passed mint account equals
    `StakingConfig.stake_mint`. Mint reassignment is admin-only; recorded
    in the `StakeMintSet` event.

S5. **Burn-ladder split.** `process_routing_fee_burn(amount)` burns
    `amount / 2` via `token::burn` (true SPL supply reduction) and
    transfers `amount - amount/2` to the staker reward pool. Odd-amount
    rounding rounds **up** to stakers, not the burn — protocol
    distribution is favoured over deflation by one base unit per call.
    Verified by `burn_ladder.rs::process_routing_fee_burn_*` tests (9 cases).

S6. **One-vote-one-burn.** `cast_reputation_vote` atomically calls
    `token::burn(VOTE_BURN_AMOUNT)` on the voter's ATA before emitting
    `ReputationVoted`. If the burn fails (no balance, wrong mint), the
    vote does not emit. Verified by
    `cast_reputation_vote.rs::happy_path` and
    `cast_reputation_vote.rs::rejects_when_stake_mint_unset`.

### Vault registry

V1. **Owner-gated configure.** `configure_vault`, `set_vault_auditor_key`,
    `set_vault_buyer_key_rotation_cid` all assert the signer equals
    `Provider.owner`. Non-owner reverts with `NotProviderOwner`. Verified
    by `vault_registry.rs::{configure_rejects_non_owner,
    set_auditor_key_rejects_attacker, set_buyer_key_rotation_cid_rejects_attacker}`.

V2. **Admin-gated deactivation.** `deactivate_vault` is gated to
    `Config.authority`, not `Provider.owner`. This mirrors the EVM
    `DEFAULT_ADMIN_ROLE` semantics — providers can re-configure but only
    the protocol can take a vault offline. Verified by
    `vault_registry.rs::deactivate_rejects_non_admin`.

V3. **Re-configure preserves rotation state.** A second `configure_vault`
    call overwrites `tier`, `on_chain_verifier`, `arweave_cid`, and
    `active`; it preserves `auditor_key` and `buyer_key_rotation_cid`.
    This lets a provider rotate their verifier program id without
    re-running their auditor or key-rotation setup. Verified by
    `vault_registry.rs::configure_preserves_auditor_and_rotation`.

V4. **Inactive vault refuses mutations.** Once `active == false`,
    `set_vault_auditor_key`, `set_vault_buyer_key_rotation_cid`, and
    `deactivate_vault` revert with `VaultInactive`. The vault is brought
    back online by a new `configure_vault` call (owner-gated).

### Vault proof dispatch

P1. **Signals-hash commitment.** `submit_vault_proof` asserts
    `signals_hash == sha256(public_signals)`. The PDA seed for the
    `VaultProofRecord` is `[b"vault_proof", provider_id, signals_hash]`,
    so a hash mismatch would let the caller create distinct records for
    the same actual proof inputs. The check makes the seed honest;
    failure surfaces as `SignalsHashMismatch`. Verified by
    `submit_vault_proof.rs::submit_vault_proof_rejects_signals_hash_mismatch`.

P2. **Vault-proof replay guard.** The `VaultProofRecord` PDA is created
    with `init` (not `init_if_needed`). A second submit with the same
    `(provider_id, signals_hash)` reverts with `AccountAlreadyInUse`
    from the system program — the existing PDA is the replay receipt.
    Verified by `submit_vault_proof.rs::submit_vault_proof_rejects_replay`.

P3. **Verifier-program account binding.** The handler asserts
    `verifier_program.key() == VaultConfig.on_chain_verifier` before any
    CPI. Passing a different program id (e.g. one that always returns
    Ok) reverts with `VerifierProgramMismatch`. Verified by
    `submit_vault_proof.rs::submit_vault_proof_rejects_wrong_verifier_program`.

P4. **Unset verifier rejection.** If `VaultConfig.on_chain_verifier ==
    Pubkey::default()`, the handler reverts with `VerifierUnset` before
    any CPI. Lets a provider register a non-Open tier while their real
    verifier program is still under audit, without exposing a "no proof
    required" hole. Verified by
    `submit_vault_proof.rs::submit_vault_proof_rejects_unset_verifier`.

P5. **Open-tier rejection.** If `VaultConfig.tier == Open`, the handler
    reverts with `TierDoesNotRequireProof`. Cleanly separates the
    no-cryptography tier from the proof-required tiers. Verified by
    `submit_vault_proof.rs::submit_vault_proof_rejects_open_tier`.

P6. **Inactive vault rejection.** Submitting against a deactivated vault
    reverts with `VaultInactive`.

P7. **CPI failure propagation.** Any non-Ok return from the verifier
    program CPI maps to `ProofVerificationFailed` and the on-chain state
    is unchanged (no `VaultProofRecord` is created, no event is emitted).
    Verified by `submit_vault_proof.rs::submit_vault_proof_rejects_buyer_key_stub`.

P8. **Buyer-key stub is universally rejecting.** Today the
    `vault_verifier_buyer_key` program rejects every input with
    `CircuitNotFinalised`. There is no path through it that returns Ok.
    `submit_vault_proof` against a `BuyerKey`-tier vault therefore always
    reverts until the real circuit ships.

### Cryptographic surface

C1. **Ed25519 precompile only via lookback.** The `ed25519_verify` module
    loads the **preceding** instruction in the same transaction via the
    instructions sysvar, asserts its `program_id == ed25519_program::ID`,
    parses the precompile's serialised payload, and compares
    `(pubkey, message)` against the expected values. The precompile is
    not directly callable from the program (Solana's Ed25519 precompile
    is a separate program that runs at tx-validation time, not via CPI);
    the lookback pattern is the canonical way to consume it.

C2. **Precompile payload validation.** The verifier asserts:
    - `num_signatures == 1` (the protocol does not support multi-sig vouchers).
    - All three `*_instruction_index` fields equal `u16::MAX` (the data
      is inside the precompile's own ix, not cross-referenced into
      another tx ix — preventing a future ix from supplying the bytes).
    - All offsets land inside the precompile's data buffer.
    Failures map to `MissingPrecompile`. The handler does not trust the
    precompile to have validated structural soundness — it re-parses.

C3. **Groth16 verification.** The `vault-verifier-core` crate wraps
    `groth16-solana::Groth16Verifier`. Verifying-key bytes follow the
    BN254 big-endian / c1-first G2-limb convention required by Solana's
    `alt_bn128` syscalls. Callers (the per-circuit verifier programs)
    pass a 256-byte proof in `[A(64) || B(128) || C(64)]` layout. The
    off-chain proof emitter pre-negates `proof.pi_a.y` per the snarkjs
    convention — this is documented in
    `vault-verifier-core/src/lib.rs` and enforced by the fixture-based
    unit tests (3 tests including `rejects_tampered_proof` and
    `rejects_wrong_public_inputs`).

C4. **Manual CPI dispatch surface.** `submit_vault_proof` constructs the
    verifier-program ix data manually as
    `[discriminator(8) || borsh(proof_bytes, public_signals)]`. The
    discriminator is the hardcoded const `VERIFY_IX_DISCRIMINATOR` =
    `sha256("global:verify")[..8]`. All three verifier programs expose
    the same `verify` entrypoint, so one constant covers every per-circuit
    dispatch. If a future verifier renames the entrypoint, this constant
    must change.

### Token-mint binding (escrow side)

T1. **USDC mint pinned at init.** `UsdcConfig.usdc_mint` is set on
    `init_escrow` and is never mutated. Every escrow path
    (`open_session`, `settle`, `close_session`) asserts the passed mint
    account equals `UsdcConfig.usdc_mint`. Cross-mint attacks (proving
    cumulative spend in a worthless mint and draining the USDC vault)
    are not reachable because the program signs token transfers as the
    PDA authority — only the configured mint's ATAs match the PDA
    derivation.

T2. **`session_vault_ata` and `routing_fee_vault_ata` are PDA-owned.**
    The `session_vault_authority` and `routing_fee_vault_authority` PDAs
    own their respective ATAs; no off-chain key signs the outflow.

### x402 receipts

X1. **Receipt records are immutable.** `record_x402_receipt` creates a
    `[b"x402", session_id, provider_id]` PDA with `init`. The PDA is
    write-once; subsequent re-records for the same session-provider pair
    revert via account-already-in-use. The receipt does not move tokens
    (it is a proof-of-payment audit trail; payment happened off-chain
    through the x402 HTTP flow).

X2. **No authority gating.** Anyone can call `record_x402_receipt` to
    register a receipt. This is intentional — the receipt is a
    public audit trail, and the off-chain payment proof is whatever the
    HTTP layer demanded. No on-chain trust is granted on the basis of a
    receipt existing.

## Constants

| Constant | Value | Source | Risk if wrong |
|---|---|---|---|
| `MIN_STAKE_DEFAULT` | `50_000_000` (50 $GAZE at 6 decimals) | `lib.rs:10` | Lower stake floor; rebakeable via `init_staking` |
| `VERIFIED_STAKE_DEFAULT` | `500_000_000` (500 $GAZE) | `lib.rs:14` | Lower verified threshold; rebakeable |
| `COOLDOWN_DEFAULT_SECS` | `604_800` (7 days) | `lib.rs:17` | Shorter cooldown weakens slash window; rebakeable |
| `VOTE_BURN_AMOUNT` | `1_000_000` (1 $GAZE) | `lib.rs:22` | **Hard-coded** — redeploy required to change. Confirm at $GAZE launch |
| `ROUTING_FEE_BPS` | `200` (2%) | `lib.rs:46` | **Hard-coded** — redeploy required to change |
| `BURN_DESTINATION` | fixed Pubkey | `lib.rs:26` | **Hard-coded** — redeploy required to change |
| `VOUCHER_DOMAIN_TAG` | `"StargazeMPP/Voucher/1"` | `lib.rs:36` | Changing breaks all existing voucher signatures |
| `VERIFY_IX_DISCRIMINATOR` | sha256("global:verify")[..8] | `lib.rs:62` | Must match the verifier programs' Anchor discriminator |

## Known unfinished surfaces

- **Buyer-key circuit.** The `vault_verifier_buyer_key` program is a
  rejecting stub. No `BuyerKey`-tier vault can produce a valid proof
  today.
- **Dev vkeys in all three verifier programs.** Generated by
  `vault-circuits/scripts/setup-dev.mjs` with hard-coded entropy. Must
  be re-baked from a real multi-contributor ceremony before mainnet;
  see [`docs/vault-ceremony.md`](./docs/vault-ceremony.md).
- **Staker reward distribution.** `process_routing_fee_burn` accumulates
  the staker share into `[b"staker_reward_pool"]`. The claim mechanism
  (pull-based Merkle vs push-based proportional) is unresolved; nothing
  on-chain reads from this pool today.
- **`convert_routing_fees_to_gaze` (USDC → $GAZE swap).** Not yet
  implemented; routing fees accumulate as USDC. Blocked on the $GAZE
  mint address and a swap-router choice (Jupiter CPI is the planned
  path).
- **Upgrade-authority decisions.** Per-program upgrade authority for
  each verifier program is operator-pending. Recommendation:
  immutable-after-deploy. See
  [`docs/vault-verifier-deployment.md`](./docs/vault-verifier-deployment.md).

## Reporting

Report vulnerabilities privately to the operator. **Do not** open public
GitHub issues for security bugs. The Immunefi bounty programme will be
listed before mainnet; until then the disclosure channel is direct
contact with the operator and the Trail of Bits engagement.
