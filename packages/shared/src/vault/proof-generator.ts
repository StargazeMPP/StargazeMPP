export interface VaultProofInput {
  /** Private witness inputs — fetched from the provider's secure endpoint. Never leaves the prover. */
  privateInputs: Record<string, unknown>;
  /** Public query parameters — visible to the agent and committed in the proof. */
  publicParams: Record<string, unknown>;
}

export interface VaultProofOutput {
  /** The public signals the agent receives. */
  publicOutput: unknown[];
  /** keccak256 of the serialized proof — short and on-chain-friendly. */
  proofHash: string;
  /** Arweave CID where the full proof bytes are stored (verifiable by anyone, forever). */
  proofArweaveCid?: string;
}

/**
 * Generates and verifies Groth16 proofs for vault-tier queries. Implemented
 * by this team via snarkjs + circom; consumed by the backend's
 * `StargazeVault` pipeline and by `@stargazempp/provider-sdk`'s
 * `vaultMonetize` decorator.
 */
export interface VaultProofGenerator {
  generate(circuitId: string, inputs: VaultProofInput): Promise<VaultProofOutput>;
  verify(circuitId: string, output: VaultProofOutput): Promise<boolean>;
}
