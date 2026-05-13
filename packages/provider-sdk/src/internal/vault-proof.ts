import { createHash } from 'node:crypto';
import { PublicKey, SystemProgram, TransactionInstruction } from '@solana/web3.js';

/**
 * Default base58-encoded program id for `stargaze_anchor`. Mirrors the
 * `declare_id!` macro in `packages/anchor-program/programs/stargaze_anchor/src/lib.rs`.
 * Callers can override via `params.stargazeProgramId` when targeting a
 * non-default deploy (e.g. a feature-branch program).
 */
export const STARGAZE_ANCHOR_PROGRAM_ID = new PublicKey(
  'm6P7kwvXoET9n5B8DFGwwLEozXdv6jBJPdbMiW1TH1R',
);

/**
 * Anchor `global:submit_vault_proof` ix discriminator —
 * `sha256("global:submit_vault_proof")[..8]`. Hardcoded so the helper has
 * no runtime hash cost and no `@coral-xyz/anchor` dep — the IDL mirror in
 * `@stargazempp/shared` lists the same bytes.
 */
export const SUBMIT_VAULT_PROOF_DISCRIMINATOR = new Uint8Array([
  0xad, 0x19, 0x0d, 0x0a, 0x7f, 0xe0, 0xcf, 0xf5,
]);

const PROVIDER_ID_LEN = 32;
const SIGNALS_HASH_LEN = 32;
const PROOF_BYTES_LEN = 256;
const SIGNAL_LEN = 32;

export interface SubmitVaultProofParams {
  /** Signer who pays rent for the freshly-created `VaultProofRecord` PDA. */
  submitter: PublicKey;
  /** Program id of the per-circuit Groth16 verifier — must equal
   *  `VaultConfig.on_chain_verifier` set by the provider via `configure_vault`. */
  verifierProgramId: PublicKey;
  /** 32-byte provider id (raw bytes, not base58). */
  providerId: Uint8Array;
  /** 256-byte Groth16 proof. Caller is responsible for snarkjs → Solana
   *  preprocessing: negate `proof.pi_a.y` and reorder G2 limbs to c1-first. */
  proofBytes: Uint8Array;
  /** Public signals, each a 32-byte big-endian field element. */
  publicSignals: Uint8Array[];
  /** Override `stargaze_anchor` program id (defaults to the localnet/devnet
   *  declared id). */
  stargazeProgramId?: PublicKey;
}

export interface SubmitVaultProofResult {
  /** Ready-to-send `TransactionInstruction`. */
  instruction: TransactionInstruction;
  /** sha256 of the concatenated public signals (32 bytes). Same value used
   *  for the `vault_proof_record` PDA seed. */
  signalsHash: Uint8Array;
  /** Address of the `VaultProofRecord` PDA that this ix will initialise. */
  proofRecordPda: PublicKey;
  /** Address of the `VaultConfig` PDA the ix reads from. */
  vaultConfigPda: PublicKey;
}

/**
 * sha256 of the concatenated 32-byte public-signal limbs. Matches the
 * on-chain `signals_hash` derivation in `submit_vault_proof` so callers can
 * pre-compute the `VaultProofRecord` PDA address before sending.
 */
export function computeVaultProofSignalsHash(publicSignals: Uint8Array[]): Uint8Array {
  const hasher = createHash('sha256');
  for (const [i, sig] of publicSignals.entries()) {
    if (sig.length !== SIGNAL_LEN) {
      throw new Error(
        `computeVaultProofSignalsHash: publicSignals[${i}] must be ${SIGNAL_LEN} bytes, got ${sig.length}`,
      );
    }
    hasher.update(sig);
  }
  return new Uint8Array(hasher.digest());
}

/**
 * Derive the `VaultConfig` PDA — seeds `[b"vault", provider_id]`.
 */
export function deriveVaultConfigPda(
  providerId: Uint8Array,
  stargazeProgramId: PublicKey = STARGAZE_ANCHOR_PROGRAM_ID,
): PublicKey {
  if (providerId.length !== PROVIDER_ID_LEN) {
    throw new Error(
      `deriveVaultConfigPda: providerId must be ${PROVIDER_ID_LEN} bytes, got ${providerId.length}`,
    );
  }
  const [pda] = PublicKey.findProgramAddressSync(
    [Buffer.from('vault'), Buffer.from(providerId)],
    stargazeProgramId,
  );
  return pda;
}

/**
 * Derive the `VaultProofRecord` PDA — seeds
 * `[b"vault_proof", provider_id, signals_hash]`. The PDA is `init`-only on
 * the program side, so collision = replay-rejected.
 */
export function deriveVaultProofRecordPda(
  providerId: Uint8Array,
  signalsHash: Uint8Array,
  stargazeProgramId: PublicKey = STARGAZE_ANCHOR_PROGRAM_ID,
): PublicKey {
  if (providerId.length !== PROVIDER_ID_LEN) {
    throw new Error(
      `deriveVaultProofRecordPda: providerId must be ${PROVIDER_ID_LEN} bytes, got ${providerId.length}`,
    );
  }
  if (signalsHash.length !== SIGNALS_HASH_LEN) {
    throw new Error(
      `deriveVaultProofRecordPda: signalsHash must be ${SIGNALS_HASH_LEN} bytes, got ${signalsHash.length}`,
    );
  }
  const [pda] = PublicKey.findProgramAddressSync(
    [Buffer.from('vault_proof'), Buffer.from(providerId), Buffer.from(signalsHash)],
    stargazeProgramId,
  );
  return pda;
}

/**
 * Build the `submit_vault_proof` `TransactionInstruction`. The caller is
 * responsible for wrapping it in a `Transaction`, attaching a compute-unit
 * budget (recommend at least 600k CU to cover the verifier-program CPI),
 * and signing as `submitter` before sending.
 *
 * The handler verifies that `signals_hash` matches `sha256(public_signals)`
 * — passing a wrong commitment surfaces as `SignalsHashMismatch` and the
 * ix reverts before any CPI.
 */
export function buildSubmitVaultProofInstruction(
  params: SubmitVaultProofParams,
): SubmitVaultProofResult {
  if (params.providerId.length !== PROVIDER_ID_LEN) {
    throw new Error(
      `buildSubmitVaultProofInstruction: providerId must be ${PROVIDER_ID_LEN} bytes, got ${params.providerId.length}`,
    );
  }
  if (params.proofBytes.length !== PROOF_BYTES_LEN) {
    throw new Error(
      `buildSubmitVaultProofInstruction: proofBytes must be ${PROOF_BYTES_LEN} bytes, got ${params.proofBytes.length}`,
    );
  }
  for (const [i, sig] of params.publicSignals.entries()) {
    if (sig.length !== SIGNAL_LEN) {
      throw new Error(
        `buildSubmitVaultProofInstruction: publicSignals[${i}] must be ${SIGNAL_LEN} bytes, got ${sig.length}`,
      );
    }
  }

  const programId = params.stargazeProgramId ?? STARGAZE_ANCHOR_PROGRAM_ID;
  const signalsHash = computeVaultProofSignalsHash(params.publicSignals);
  const vaultConfigPda = deriveVaultConfigPda(params.providerId, programId);
  const proofRecordPda = deriveVaultProofRecordPda(
    params.providerId,
    signalsHash,
    programId,
  );

  // Borsh layout for `SubmitVaultProof { provider_id, signals_hash,
  // proof_bytes, public_signals }`:
  //   [8 disc][32 provider_id][32 signals_hash][256 proof_bytes]
  //   [4 vec-len LE][N * 32 public_signals]
  const signalsLen = params.publicSignals.length;
  const dataLen = 8 + 32 + 32 + 256 + 4 + signalsLen * SIGNAL_LEN;
  const data = new Uint8Array(dataLen);
  let cursor = 0;
  data.set(SUBMIT_VAULT_PROOF_DISCRIMINATOR, cursor);
  cursor += 8;
  data.set(params.providerId, cursor);
  cursor += 32;
  data.set(signalsHash, cursor);
  cursor += 32;
  data.set(params.proofBytes, cursor);
  cursor += 256;
  // Vec length (u32, little-endian).
  const view = new DataView(data.buffer, data.byteOffset, data.byteLength);
  view.setUint32(cursor, signalsLen, true);
  cursor += 4;
  for (const sig of params.publicSignals) {
    data.set(sig, cursor);
    cursor += SIGNAL_LEN;
  }

  const instruction = new TransactionInstruction({
    programId,
    keys: [
      { pubkey: params.submitter, isSigner: true, isWritable: true },
      { pubkey: vaultConfigPda, isSigner: false, isWritable: false },
      { pubkey: params.verifierProgramId, isSigner: false, isWritable: false },
      { pubkey: proofRecordPda, isSigner: false, isWritable: true },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data: Buffer.from(data),
  });

  return { instruction, signalsHash, proofRecordPda, vaultConfigPda };
}
