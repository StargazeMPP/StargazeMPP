/**
 * Minimal ambient typings for snarkjs — the published package ships
 * runtime-only without `.d.ts`. We only annotate the surface the proof
 * generator + dev-setup script consume.
 */
declare module 'snarkjs' {
  export const groth16: {
    fullProve: (
      input: Record<string, unknown>,
      wasmPath: string,
      zkeyPath: string,
    ) => Promise<{ proof: unknown; publicSignals: unknown[] }>;
    verify: (
      verificationKey: Record<string, unknown>,
      publicSignals: Array<string | number | bigint>,
      proof: unknown,
    ) => Promise<boolean>;
  };

  export const powersOfTau: {
    newAccumulator: (curve: string, power: number, outPath: string) => Promise<unknown>;
    contribute: (
      inPath: string,
      outPath: string,
      name: string,
      entropy: string,
    ) => Promise<unknown>;
    preparePhase2: (inPath: string, outPath: string) => Promise<unknown>;
  };

  export const zKey: {
    newZKey: (r1cs: string, ptau: string, outPath: string) => Promise<unknown>;
    contribute: (
      inPath: string,
      outPath: string,
      name: string,
      entropy: string,
    ) => Promise<unknown>;
    exportVerificationKey: (zkeyPath: string) => Promise<Record<string, unknown>>;
  };
}
