import type { Address } from 'viem';

export interface DeployedAddresses {
  StargazeEscrow: Address;
  StargazeRegistry: Address;
  StubStakeChecker: Address;
  PrivacyVaultRegistry: Address;
  StargazeCcipReceiver: Address;
  StargazeStakeMirror?: Address;
  AggregateSumVerifier?: Address;
  AggregateMeanVerifier?: Address;
  GeofenceVerifier?: Address;
}

export type TempoNetwork = 'tempo-mainnet' | 'tempo-testnet' | 'local';

/**
 * Populated by `packages/contracts-evm`'s deploy script. Empty here until
 * the first testnet deploy lands. The three Groth16 verifier fields
 * (AggregateSumVerifier, AggregateMeanVerifier, GeofenceVerifier) are
 * optional: they are shared instances across providers and may not be
 * present on every network.
 *
 * `StargazeStakeMirror` is also optional: the registry's day-one
 * `IStakeChecker` is `StubStakeChecker`, and the admin swaps in the
 * Solana-to-Tempo stake mirror post-launch by calling
 * `StargazeRegistry.setStakeChecker(mirror)`. The address is only set on
 * networks where that swap has happened.
 */
export const ADDRESSES: Partial<Record<TempoNetwork, DeployedAddresses>> = {};

export function getAddresses(network: TempoNetwork): DeployedAddresses {
  const addrs = ADDRESSES[network];
  if (!addrs) {
    throw new Error(
      `StargazeMPP contracts not yet deployed on ${network}. Update packages/shared/src/evm/addresses.ts after the deploy script runs.`,
    );
  }
  return addrs;
}
