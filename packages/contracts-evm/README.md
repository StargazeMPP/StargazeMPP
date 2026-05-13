# `@stargazempp/contracts-evm`

Solidity contracts deployed to Tempo EVM. Token logic (staking, slashing, burns) lives on Solana; these contracts hold settlement, escrow, reputation, and vault config.

| Contract | Role |
|---|---|
| [`StargazeEscrow`](src/StargazeEscrow.sol) | PathUSD session escrow + EIP-712 cumulative voucher batch settlement with on-chain monotonicity. 2% routing fee forwarded to a treasury address; auto-refunds unused balance in the settlement transaction. |
| [`StargazeRegistry`](src/StargazeRegistry.sol) | Provider registration (no stake held locally), reputation score storage, Verified Provider tier. Delegates the stake check to an `IStakeChecker` so the verified tier reflects Solana-side `$GAZE` stake. |
| [`IStakeChecker`](src/IStakeChecker.sol) + [`StubStakeChecker`](src/StubStakeChecker.sol) | Temporary delegate that gates `isVerified` against an admin-controlled allowlist. Production wires a CCIP-mirrored Solana stake oracle. |
| [`PrivacyVaultRegistry`](src/PrivacyVaultRegistry.sol) | Per-provider Groth16 verifier addresses, buyer-key rotation policy, auditor-key management for confidential payments. |
| [`StargazeCcipReceiver`](src/StargazeCcipReceiver.sol) | Chainlink CCIP receiver that mirrors Solana-emitted reputation scores back into `StargazeRegistry`. |
| Groth16 verifiers | [`AggregateSumVerifier`](src/AggregateSumVerifier.sol), [`AggregateMeanVerifier`](src/AggregateMeanVerifier.sol), [`GeofenceVerifier`](src/GeofenceVerifier.sol) — auto-generated on-chain verifiers for the vault circuits. |

## Build & test

```bash
forge install
forge build
forge test
```

## Deploy

`script/Deploy.s.sol` deploys the Tempo surface: `StargazeEscrow` → `StargazeRegistry` (+ `StubStakeChecker`) → `PrivacyVaultRegistry` → `StargazeCcipReceiver`. Pass the multisig admin address, PathUSD token address, and routing-fee treasury address via environment.

```bash
DEPLOYER_PRIVATE_KEY=… ADMIN_MULTISIG=0x… PATHUSD_ADDRESS=0x… ROUTING_TREASURY=0x… \
  forge script script/Deploy.s.sol --rpc-url $TEMPO_TESTNET_RPC --broadcast
```

## Standards

- Solc 0.8.27, via-IR + optimiser (200 runs).
- 4-of-7 Safe multisig upgrade authority + 14-day timelock on every Tempo EVM contract.
- Trail of Bits audit before mainnet.
- Certora formal verification on `StargazeEscrow` voucher-settlement invariants.

## Publish ABIs

After `forge build`, regenerate the typed ABI bundle consumed by `@stargazempp/shared`:

```bash
node scripts/publish-abis.mjs
```
