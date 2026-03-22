# `packages/contracts-evm` — Tempo EVM contracts

Solidity contracts. Foundry: `forge install && forge build && forge test`.

Today:
- `GAZEToken` — ERC-20 with staking + 7-day unstake cooldown + transfer hook into `BurnController`.
- `BurnController` — routing-fee burn (50/50 split with staker pool), citation burn (5 GAZE), reputation-vote burn (1 GAZE).

Incoming:
- `StargazeEscrow` — MPP session escrow + voucher batch settlement.
- `StargazeRegistry` — provider stake + reputation storage + slashing.
- `PrivacyVaultRegistry` — per-provider ZK verifier config.

Standards target: 4-of-7 Safe multisig upgrade authority, 14-day timelock, Trail of Bits audit before mainnet, Certora formal verification on the `GAZEToken` transfer hook.
