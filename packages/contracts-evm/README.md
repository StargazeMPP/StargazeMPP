# `@stargazempp/contracts-evm`

Solidity contracts deployed to Tempo EVM.

| Contract | Role |
|---|---|
| [`GAZEToken`](src/GAZEToken.sol) | ERC-20 coordination token with staking (7-day unstake cooldown) and a transfer hook for routing-event telemetry. |
| [`BurnController`](src/BurnController.sol) | Atomic burn primitives — routing-fee burn (50/50 burn-vs-stakers), citation burn (5 GAZE), reputation-vote burn (1 GAZE). |
| [`StargazeEscrow`](src/StargazeEscrow.sol) | MPP session escrow + EIP-712 cumulative voucher batch settlement with on-chain monotonicity. Auto-refunds unused balance in the settlement transaction. |
| [`StargazeRegistry`](src/StargazeRegistry.sol) | Provider registration, `$GAZE` stake collection and slashing, reputation score storage, Verified Provider tier. |
| [`PrivacyVaultRegistry`](src/PrivacyVaultRegistry.sol) | Per-provider Groth16 verifier addresses, buyer-key rotation policy, auditor-key management for confidential payments. |

## Build & test

```bash
forge install
forge build
forge test
```

## Deploy

`script/Deploy.s.sol` enforces strict dependency order: `GAZEToken` → `BurnController` → `StargazeEscrow` → `StargazeRegistry` → `PrivacyVaultRegistry`. Pass the multisig admin address and PathUSD token address via environment.

```bash
DEPLOYER_PRIVATE_KEY=… ADMIN_MULTISIG=0x… PATHUSD_ADDRESS=0x… \
  forge script script/Deploy.s.sol --rpc-url $TEMPO_TESTNET_RPC --broadcast
```

## Standards

- Solc 0.8.27, via-IR + optimiser (200 runs).
- 4-of-7 Safe multisig upgrade authority + 14-day timelock on every Tempo EVM contract.
- Trail of Bits audit before mainnet.
- Certora formal verification on the `GAZEToken` transfer hook.

## Publish ABIs

After `forge build`, regenerate the typed ABI bundle consumed by `@stargazempp/shared`:

```bash
node scripts/publish-abis.mjs
```
