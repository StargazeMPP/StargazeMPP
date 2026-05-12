# `packages/contracts-evm` — Tempo EVM contracts

**Owner:** this team.

Solidity contracts deployed in the order from the backend PDF section 2:

1. `GAZEToken` — ERC-20 with transfer hook → `BurnController`, 7-day unstake cooldown, weekly reward distribution.
2. `BurnController` — atomic burns for routing fees (50% of 2%), citation burns (5 GAZE), reputation vote burns (1 GAZE).
3. `StargazeEscrow` — MPP session escrow deposits + voucher batch settlement.
4. `StargazeRegistry` — provider registration, `$GAZE` stake collection + slashing, reputation score storage, Verified badge issuance.
5. `PrivacyVaultRegistry` — per-provider ZK verifier addresses, buyer-key rotation config, auditor key management.

Standards: 4-of-7 Safe multisig upgrade authority, 14-day timelock, Trail of Bits audit before mainnet, Certora formal verification on `GAZEToken` transfer hook.
