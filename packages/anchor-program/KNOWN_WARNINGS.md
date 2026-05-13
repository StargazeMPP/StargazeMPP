# Known build warnings

Snapshot of the warnings `cargo build` surfaces for the four programs in
this crate, with rationale for not chasing them right now.

## Status as of 2026-05-13

```
warning: `stargaze_anchor`             generated 29 warnings (5 duplicates)
warning: `vault_verifier_aggregate_sum` generated 8 warnings (5 duplicates)
warning: `vault_verifier_buyer_key`    generated 8 warnings (5 duplicates)
warning: `vault_verifier_geofence`     generated 8 warnings (5 duplicates)
```

Two distinct warning classes:

1. `use of deprecated method AccountInfo::realloc: Use AccountInfo::resize() instead`
   — surfaces once per program at the `#[program]` macro expansion. The
   replacement (`AccountInfo::resize`) was added in `solana-program` in
   the 2.x line; `anchor-lang 0.31.1` still emits the deprecated call
   from inside its derive macros, so we cannot silence this at the
   call-site.
2. `unexpected cfg condition value: 'anchor-debug'` — surfaces once per
   `#[derive(Accounts)]` expansion (24+ in `stargaze_anchor`, 1 in each
   verifier program). Anchor's derive emits `cfg(anchor-debug)` checks
   that Cargo doesn't recognise under the `--check-cfg` lint that Rust
   1.79+ ships on by default.

Both warnings originate from macro expansions inside `anchor-lang
0.31.1`. Neither is fixable in our source tree without an Anchor bump.

## Upgrade landscape

The handover from session 9 flagged the desire to bump to Anchor 0.32 if
released. What actually happened upstream:

- **`0.32.0` shipped 2025-10-09**, **`0.32.1` on 2025-10-10**.
- **`1.0.0-rc.x` cycle ran Dec 2025 → Mar 2026**.
- **`1.0.0` shipped 2026-04-02**; **`1.0.2` is current** (released
  2026-05-02).

So "track 0.32" is now a choice between three jumps:

| Target | Surface | Notes |
|---|---|---|
| `0.32.1` | Minor — incremental fixes over `0.31.1`. | Likely silences the `realloc` warning but won't fix the `cfg` lint (anchor-debug was still in the derives at 0.32). |
| `1.0.x` | Major — new IDL format, derive macro reworks, possible breaking changes to `Accounts` context binding, `init-if-needed` semantics, and IDL output. | Worth doing once before the audit; touches every program and the IDL mirror in `packages/shared/`. |
| stay on `0.31.1` | None. | Warnings are cosmetic; behaviour is unchanged. |

## Recommendation

Stay on `0.31.1` until the operator opens a focused upgrade session.
Reasons:

- Anchor `1.0` is a >6-month-old API surface — a multi-hour migration
  rather than a drop-in bump. Likely needs IDL-mirror refresh, possible
  `#[derive(Accounts)]` edits, and a re-run of the full litesvm test
  matrix.
- The litesvm + anchor-spl pin in `tests/Cargo.toml` and the
  `groth16-solana` dep transitively pin a `solana-program` minor that
  `anchor-lang 1.0` may not be compatible with — needs a coordinated
  bump of the whole solana toolchain.
- The current warnings are **non-blocking** — programs build, all 100
  litesvm tests pass, CU benches land well under the 1.4M default
  budget. Nothing here is shipping-critical.

When the operator does open an upgrade session, the recommended path is:

1. Cut a branch.
2. Bump every `anchor-lang = "0.31.1"` and `anchor-spl = "0.31.1"` to
   the same target (do not mix versions across programs — the IDL build
   step depends on a single anchor-syn).
3. Run `anchor build` per program; expect compile errors in
   `#[derive(Accounts)]` blocks and in any handler that still calls
   `Context::accounts.foo.realloc()`.
4. Refresh `packages/shared/src/solana/idl/*.json` from
   `target/idl/*.json`.
5. Re-run `cargo test -p stargaze_anchor_tests --tests --no-fail-fast`.
6. Re-run the two CU benches with `-- --nocapture` and update
   `BENCH.md` if the numbers shift materially.
