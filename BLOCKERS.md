# Open questions and deferred decisions

One-line entries. Resolve in the PR body that closes them; delete the line when done.

## Pre-week-1 — for internal team to answer

- [ ] Tempo testnet target — RPC URL + chain ID for `.env.example`.
- [ ] `packages/shared/evm/abi/` — when do placeholder ABIs land for `GAZEToken`, `StargazeEscrow`, `StargazeRegistry`?
- [ ] `MppVerifier.verifyDeposit` exact signature — `(txHash, expectedRecipient, minAmount) → { amount, agentWallet }`?
- [ ] JWT signing key strategy — env for testnet, KMS for mainnet?
- [ ] Reputation Oracle on-chain commit — payment-router wallet or its own key?
- [ ] M1 Solana-side `session.query` (x402 USDC) — shared scope or internal-only for M1?

## Identity / infra

- [ ] Add `~/.ssh/stargazempp_oskarpetri.pub` to the `oskarpetri` GitHub account (Settings → SSH and GPG keys → New SSH key, title "StargazeMPP / dennisgoslar mac"). Public key value is printed by this session — paste from there.
- [ ] `gh auth login` inside this repo (direnv picks `GH_CONFIG_DIR=~/.config/gh-stargazempp` automatically) and sign in as `oskarpetri`.
- [ ] Confirm whether `StargazeMPP/StargazeMPP` GitHub repo exists yet; if not, create it under the `StargazeMPP` org and push.
- [ ] External dev GitHub username — needed to invite them as a collaborator on `StargazeMPP/StargazeMPP` (org-level invite preferred so they see the whole monorepo).
- [ ] Decide: is there a third spec doc `stargazempp-backend-build.pdf` (~466 KB) in `~/Downloads/` that should also be ingested into `docs/`? It was not requested but exists.

## Open product

- [ ] Naming: are the "Verified Provider tier" thresholds in overview PDF §6 (score >800 AND stake ≥500 GAZE) the final values, or stand-in?
- [ ] Naming: backend PDF §6 has `privacy_tier ∈ {OPEN, ZK-VAULT, CONFIDENTIAL, BUYER-KEY}`, overview PDF §8 SDK example uses `'OPEN' | 'zk-aggregate' | 'confidential' | 'buyer-key'`. Pick one casing and one set of identifiers.
