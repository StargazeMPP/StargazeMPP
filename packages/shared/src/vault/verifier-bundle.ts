import type { Address } from 'viem';
import type { PrivacyTier } from './tiers.js';

/**
 * Everything an agent or auditor needs to independently verify a vault
 * provider's ZK proof. Published into Arweave (Irys) on provider
 * registration; mirrored on-chain via `PrivacyVaultRegistry`.
 */
export interface Groth16VerifierBundle {
  /** Tier this circuit serves. */
  tier: PrivacyTier;
  /** Stable identifier — `${provider.id}/${circuitVersion}`. */
  circuitId: string;
  /** Verifying key (Groth16) as a serialised JSON blob. */
  verifyingKey: Record<string, unknown>;
  /** JSON Schema describing the public output array (`publicSignals`). */
  publicOutputSchema: Record<string, unknown>;
  /**
   * Arweave content ID where the full bundle (vkey + wasm + circuit source)
   * is permanently stored. Optional pre-deploy, required at registration.
   */
  arweaveCid?: string;
  /**
   * Address of the on-chain verifier contract registered with
   * `PrivacyVaultRegistry`. Optional pre-deploy.
   */
  onChainVerifier?: Address;
}
