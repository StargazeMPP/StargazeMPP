import { groth16 } from 'snarkjs';
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import type {
  VaultProofGenerator,
  VaultProofInput,
  VaultProofOutput,
} from '@stargazempp/shared';

export interface CircuitArtifacts {
  /** Filesystem path to `<name>_js/<name>.wasm` produced by `circom`. */
  wasm: string;
  /** Filesystem path to the final `.zkey` produced by the Phase-2 ceremony. */
  zkey: string;
  /** Verifying key JSON exported via `snarkjs zkey export verificationkey`. */
  verifyingKeyJsonPath: string;
}

/**
 * snarkjs / Groth16-backed implementation of `VaultProofGenerator`.
 *
 * Each registered `circuitId` maps to a triple of artifacts on disk:
 * the compiled `.wasm` witness generator, the Phase-2 `.zkey`, and the
 * exported verifying-key JSON. Loading happens lazily — the first call
 * to `generate` / `verify` for a circuit pulls the verifying key into
 * memory and caches it.
 *
 * `proofHash` is the SHA-256 of the canonical JSON serialisation of the
 * proof object. It's deterministic across machines for the same proof
 * bytes, so it can be embedded in receipts and verified later.
 */
export class SnarkjsVaultProofGenerator implements VaultProofGenerator {
  private readonly circuits: Map<string, CircuitArtifacts>;
  private readonly vkeyCache = new Map<string, Record<string, unknown>>();

  constructor(circuits: Record<string, CircuitArtifacts>) {
    this.circuits = new Map(Object.entries(circuits));
  }

  registerCircuit(id: string, artifacts: CircuitArtifacts): void {
    this.circuits.set(id, artifacts);
    this.vkeyCache.delete(id);
  }

  async generate(circuitId: string, inputs: VaultProofInput): Promise<VaultProofOutput> {
    const artifacts = this.requireArtifacts(circuitId);
    const witnessInputs = { ...inputs.privateInputs, ...inputs.publicParams };
    const { proof, publicSignals } = await groth16.fullProve(
      witnessInputs as Record<string, unknown>,
      artifacts.wasm,
      artifacts.zkey,
    );
    return {
      publicOutput: publicSignals as unknown[],
      proofHash: SnarkjsVaultProofGenerator.hashProof(proof),
    };
  }

  async verify(circuitId: string, output: VaultProofOutput, proof?: unknown): Promise<boolean> {
    if (!proof) {
      throw new Error(
        `SnarkjsVaultProofGenerator.verify: pass the raw proof object alongside the public output. ` +
          `(circuitId=${circuitId})`,
      );
    }
    const vkey = this.loadVerifyingKey(circuitId);
    const isValid = await groth16.verify(
      vkey,
      output.publicOutput as Array<string | number | bigint>,
      proof as never,
    );
    if (!isValid) return false;
    return SnarkjsVaultProofGenerator.hashProof(proof) === output.proofHash;
  }

  private requireArtifacts(circuitId: string): CircuitArtifacts {
    const a = this.circuits.get(circuitId);
    if (!a) throw new Error(`SnarkjsVaultProofGenerator: unknown circuitId '${circuitId}'`);
    return a;
  }

  private loadVerifyingKey(circuitId: string): Record<string, unknown> {
    const cached = this.vkeyCache.get(circuitId);
    if (cached) return cached;
    const a = this.requireArtifacts(circuitId);
    const raw = readFileSync(a.verifyingKeyJsonPath, 'utf-8');
    const parsed = JSON.parse(raw) as Record<string, unknown>;
    this.vkeyCache.set(circuitId, parsed);
    return parsed;
  }

  static hashProof(proof: unknown): string {
    const canonical = JSON.stringify(proof, (_key, value) =>
      typeof value === 'bigint' ? value.toString() : value,
    );
    return '0x' + createHash('sha256').update(canonical).digest('hex');
  }
}
