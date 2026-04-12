import type { Address } from 'viem';

export interface DeployedAddresses {
  GAZEToken: Address;
  BurnController: Address;
  StargazeEscrow: Address;
  StargazeRegistry: Address;
  PrivacyVaultRegistry: Address;
}

export type TempoNetwork = 'tempo-mainnet' | 'tempo-testnet' | 'local';

/**
 * Populated by `packages/contracts-evm`'s deploy script. Empty here until the
 * first testnet deploy lands — flagged in `BLOCKERS.md`.
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
