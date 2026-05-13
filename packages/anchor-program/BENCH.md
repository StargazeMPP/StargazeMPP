# Compute-unit benchmarks

Run with `--nocapture` to surface the numbers; the bench tests are
otherwise regular `cargo test` cases and always run inside the litesvm
harness:

```bash
cargo test -p stargaze_anchor_tests --test escrow_bench -- bench_settle_cu --nocapture
cargo test -p stargaze_anchor_tests --test submit_vault_proof_bench -- bench_submit_vault_proof_cu --nocapture
```

Recorded on 2026-05-13. Numbers are litesvm SBF-runtime CU and track the
Solana mainnet runtime closely; small drift between runs is expected
because secp256k1/sha256 syscalls have data-dependent costs.

## `settle` — voucher settlement

Each measurement opens a fresh session, then sends `n` cumulative vouchers
against one provider via the ed25519-precompile + program-ix pair. Per-call
CU is the **total tx CU** (precompile + program ix combined).

| n | first | last | min | max | avg |
|---|---:|---:|---:|---:|---:|
| 1 | 68,928 | 68,928 | 68,928 | 68,928 | 68,928 |
| 10 | 46,880 | 43,872 | 42,372 | 51,372 | 44,922 |
| 50 | 49,880 | 46,872 | 40,872 | 49,880 | 43,092 |
| 100 | 55,880 | 46,872 | 46,872 | 57,372 | 48,747 |
| 200 | 43,880 | 40,872 | 39,372 | 52,872 | 40,857 |

Reading:

- The first settle in a session costs ~25 k more than steady-state — Anchor
  initialises the `VoucherCursor` PDA on the first settle for that
  `(session_id, provider_id)` pair. Subsequent settles only `init` the
  per-voucher `ConsumedVoucher` PDA.
- Steady-state CU is ~40–50 k per settle, flat in `n`. The handler is
  constant-time on-chain — no iteration over prior vouchers — and the
  bench confirms it.
- Headroom: the 1.4 M default budget fits >25× steady-state settles per tx
  if a batch path is ever desired. The current single-settle path is well
  inside any compute budget the Payment Router might pick.

Implication for the off-chain Payment Router: voucher batching is **not
necessary** for CU reasons. The reason to batch — if any — is per-tx
overhead (fee, blockhash budget) rather than CU pressure.

## `submit_vault_proof` — Groth16 proof dispatch

Single happy-path proof per circuit. Each measurement is on a fresh
litesvm with all four programs loaded, so the number includes the
stargaze_anchor → verifier_program CPI overhead plus the actual pairing.

| Circuit | Public signals | CU |
|---|---:|---:|
| `aggregate_sum` | 1 | 95,545 |
| `geofence` | 4 | 109,843 |

Reading:

- Both verifiers come in well under the 600 k CU the provider-sdk helper
  recommends attaching via `ComputeBudgetInstruction::set_compute_unit_limit`,
  and ~15× under the 1.4 M default.
- Geofence is +15 k over aggregate_sum, matching the expected
  ~5 k-per-public-signal scaling for the public-input preparation step
  inside `groth16-solana`.
- The pairing check dominates — the bulk of the CU is the `alt_bn128`
  syscalls inside `groth16-solana::Groth16Verifier::verify`, not the
  Anchor / sha256 / CPI overhead.
- `buyer_key` is excluded — the verifier program is an always-rejecting
  stub and there's no meaningful CU number to record until the real
  circuit ships.

Implication for the SDK: keep the 600 k recommendation in
`provider-sdk/src/internal/vault-proof.ts`. It carries roughly 5× headroom
over the measured worst case, which is the right margin to absorb future
circuit-size growth (e.g. `aggregate_sum` with `N=16` or a more complex
buyer_key circuit) without breaking the integration.
