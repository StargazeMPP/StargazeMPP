# StargazeMPP workspace orchestration.
#
# `make check` is the unified before-commit gate: anchor build + cargo
# tests across every Rust crate + vitest across every TypeScript
# package + tsc typechecks. Sub-targets exist for iterating on a single
# stack without paying for the rest.
#
# Anchor build is the slowest step (~30-60s cold), so `make check-fast`
# is provided for the inner loop and `make check` is what runs before a
# push.

.PHONY: check check-fast check-anchor anchor-build check-rust check-ts \
        typecheck clean help

TS_PKGS := packages/provider-sdk \
           packages/reputation-oracle \
           packages/vault-circuits

# Default target prints help so a bare `make` is self-documenting.
help:
	@echo "StargazeMPP — workspace targets"
	@echo
	@echo "  make check        Full gate: anchor build, all cargo tests,"
	@echo "                    all vitest, all typecheck."
	@echo "  make check-fast   Same as check but skips anchor build."
	@echo "  make anchor-build Just \`anchor build\`."
	@echo "  make check-anchor cargo workspace tests for anchor-program"
	@echo "                    (litesvm suite + 3 verifier programs +"
	@echo "                    vault-verifier-core)."
	@echo "  make check-rust   cargo tests for indexer + stargaze-events."
	@echo "  make check-ts     vitest run for every TS package."
	@echo "  make typecheck    tsc --noEmit for shared + provider-sdk +"
	@echo "                    vault-circuits."
	@echo "  make clean        cargo clean across every Rust workspace."

check: anchor-build check-anchor check-rust check-ts typecheck
	@echo
	@echo "==> all green"

check-fast: check-anchor check-rust check-ts typecheck
	@echo
	@echo "==> all green (anchor build skipped)"

anchor-build:
	@echo "==> anchor build"
	cd packages/anchor-program && anchor build

check-anchor:
	@echo "==> cargo test (anchor-program workspace)"
	cargo test --manifest-path packages/anchor-program/Cargo.toml \
	    --workspace --tests --no-fail-fast

check-rust:
	@echo "==> cargo test (stargaze-events)"
	cargo test --manifest-path packages/stargaze-events/Cargo.toml \
	    --no-fail-fast
	@echo "==> cargo test (indexer lib + bins)"
	cargo test --manifest-path packages/indexer/Cargo.toml \
	    --lib --bins --no-fail-fast

check-ts:
	@for p in $(TS_PKGS); do \
	    echo "==> vitest run ($$p)"; \
	    (cd $$p && npx vitest run) || exit 1; \
	done

typecheck:
	@echo "==> tsc shared"
	npx tsc --noEmit -p packages/shared
	@echo "==> tsc provider-sdk (typecheck config, covers bin/)"
	npx tsc --noEmit -p packages/provider-sdk/tsconfig.typecheck.json
	@echo "==> tsc vault-circuits"
	cd packages/vault-circuits && npx tsc --noEmit

clean:
	cargo clean --manifest-path packages/anchor-program/Cargo.toml
	cargo clean --manifest-path packages/indexer/Cargo.toml
	cargo clean --manifest-path packages/stargaze-events/Cargo.toml
