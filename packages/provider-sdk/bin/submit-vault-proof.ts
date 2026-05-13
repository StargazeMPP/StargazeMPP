#!/usr/bin/env node
/**
 * `stargaze-submit-vault-proof` — CLI wrapper around
 * `buildSubmitVaultProofInstruction`. Loads a solana-CLI-style keypair
 * file, builds the ix, and sends the tx via `@solana/web3.js`. Mirrors
 * the example block in `docs/vault-verifier-deployment.md` so an operator
 * bringing up a verifier on devnet has a one-liner that doesn't require
 * writing TS glue.
 *
 * Flags:
 *   --keypair <path>           Solana JSON keypair (64-byte Uint8Array).
 *   --rpc <url>                RPC endpoint (default: devnet).
 *   --verifier <base58>        Verifier program id (must equal the
 *                              `on_chain_verifier` set by the provider).
 *   --provider-id <hex>        32-byte provider id, hex-encoded.
 *   --proof <path>             JSON file with the encoded proof bundle
 *                              (see `loadProofBundle` for the schema).
 *   --stargaze-program-id <id> Override the default stargaze_anchor id.
 *   --compute-units <n>        CU budget for the tx (default 600_000).
 *   --commitment <c>           Send commitment (default 'confirmed').
 *   --dry-run                  Build + simulate + print; don't send.
 *
 * Build via `npm run build` in this package; then invoke as
 * `node dist/bin/submit-vault-proof.js …`. Operators on bun can skip the
 * build step: `bun packages/provider-sdk/bin/submit-vault-proof.ts …`.
 */

import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import {
  ComputeBudgetProgram,
  Connection,
  Keypair,
  PublicKey,
  Transaction,
} from '@solana/web3.js';
import type { Commitment } from '@solana/web3.js';

import { buildSubmitVaultProofInstruction } from '../src/internal/vault-proof.js';

const DEFAULT_RPC = 'https://api.devnet.solana.com';
const DEFAULT_COMPUTE_UNITS = 600_000;
const DEFAULT_COMMITMENT: Commitment = 'confirmed';

const PROOF_BYTES_LEN = 256;
const PROVIDER_ID_LEN = 32;
const SIGNAL_LEN = 32;

export interface CliArgs {
  keypair: string;
  rpc: string;
  verifier: string;
  providerId: string;
  proof: string;
  stargazeProgramId?: string;
  computeUnits: number;
  commitment: Commitment;
  dryRun: boolean;
}

export interface ProofBundle {
  proofBytes: Uint8Array;
  publicSignals: Uint8Array[];
}

/**
 * Parse the CLI arg vector. Recognises `--flag value` pairs only — no
 * combined `--flag=value` syntax to keep parsing trivial. Throws on
 * unknown flags so typos surface immediately.
 */
export function parseArgs(argv: string[]): CliArgs {
  const required = ['keypair', 'verifier', 'provider-id', 'proof'] as const;
  const seen = new Map<string, string>();
  let dryRun = false;

  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (a === '--dry-run') {
      dryRun = true;
      continue;
    }
    if (a === '--help' || a === '-h') {
      console.log(usage());
      process.exit(0);
    }
    if (typeof a !== 'string' || !a.startsWith('--')) {
      throw new Error(`unexpected positional argument: ${a}`);
    }
    const key = a.slice(2);
    const value = argv[i + 1];
    if (value === undefined || value.startsWith('--')) {
      throw new Error(`flag --${key} requires a value`);
    }
    if (
      ![
        'keypair',
        'rpc',
        'verifier',
        'provider-id',
        'proof',
        'stargaze-program-id',
        'compute-units',
        'commitment',
      ].includes(key)
    ) {
      throw new Error(`unknown flag: --${key}`);
    }
    seen.set(key, value);
    i++;
  }

  for (const r of required) {
    if (!seen.has(r)) {
      throw new Error(`missing required flag: --${r}`);
    }
  }

  const computeUnitsRaw = seen.get('compute-units');
  const computeUnits = computeUnitsRaw === undefined
    ? DEFAULT_COMPUTE_UNITS
    : Number.parseInt(computeUnitsRaw, 10);
  if (!Number.isFinite(computeUnits) || computeUnits <= 0) {
    throw new Error(`--compute-units must be a positive integer, got ${computeUnitsRaw}`);
  }

  const commitment = (seen.get('commitment') ?? DEFAULT_COMMITMENT) as Commitment;

  return {
    keypair: seen.get('keypair')!,
    rpc: seen.get('rpc') ?? DEFAULT_RPC,
    verifier: seen.get('verifier')!,
    providerId: seen.get('provider-id')!,
    proof: seen.get('proof')!,
    stargazeProgramId: seen.get('stargaze-program-id'),
    computeUnits,
    commitment,
    dryRun,
  };
}

/** Strip optional `0x` prefix and lowercase. */
export function normaliseHex(input: string): string {
  const trimmed = input.trim();
  return trimmed.startsWith('0x') || trimmed.startsWith('0X')
    ? trimmed.slice(2).toLowerCase()
    : trimmed.toLowerCase();
}

/** Parse a hex string into a Uint8Array. Throws on odd length or non-hex chars. */
export function decodeHex(input: string, expectedLen?: number): Uint8Array {
  const hex = normaliseHex(input);
  if (hex.length % 2 !== 0) {
    throw new Error(`hex string has odd length: ${hex.length}`);
  }
  if (!/^[0-9a-f]*$/.test(hex)) {
    throw new Error(`hex string contains non-hex characters`);
  }
  const len = hex.length / 2;
  if (expectedLen !== undefined && len !== expectedLen) {
    throw new Error(`expected ${expectedLen} bytes, got ${len}`);
  }
  const out = new Uint8Array(len);
  for (let i = 0; i < len; i++) {
    out[i] = Number.parseInt(hex.slice(i * 2, i * 2 + 2), 16);
  }
  return out;
}

/**
 * Decode the JSON proof bundle written by the off-chain prover. Schema:
 *
 * ```json
 * {
 *   "proofHex": "<512 hex chars = 256 bytes, with optional 0x prefix>",
 *   "publicSignalsHex": [
 *     "<64 hex chars = 32 bytes>",
 *     "<64 hex chars = 32 bytes>",
 *     ...
 *   ]
 * }
 * ```
 *
 * Inputs must already be Solana-encoded (BN254 big-endian, c1-first G2,
 * pi_a.y negated). Use `packages/vault-circuits/scripts/emit-rust-vkey.mjs`
 * with `--kind fixture` as the reference encoding pipeline — the bytes it
 * embeds in the Rust fixture are the same bytes accepted here.
 */
export function loadProofBundle(path: string): ProofBundle {
  const raw = readFileSync(path, 'utf-8');
  let parsed: unknown;
  try {
    parsed = JSON.parse(raw);
  } catch (err) {
    throw new Error(`could not parse ${path} as JSON: ${(err as Error).message}`);
  }
  if (typeof parsed !== 'object' || parsed === null) {
    throw new Error(`${path} must be a JSON object`);
  }
  const { proofHex, publicSignalsHex } = parsed as {
    proofHex?: unknown;
    publicSignalsHex?: unknown;
  };
  if (typeof proofHex !== 'string') {
    throw new Error(`${path}: proofHex must be a string`);
  }
  if (!Array.isArray(publicSignalsHex)) {
    throw new Error(`${path}: publicSignalsHex must be an array`);
  }
  const proofBytes = decodeHex(proofHex, PROOF_BYTES_LEN);
  const publicSignals = publicSignalsHex.map((s, idx) => {
    if (typeof s !== 'string') {
      throw new Error(`${path}: publicSignalsHex[${idx}] must be a string`);
    }
    return decodeHex(s, SIGNAL_LEN);
  });
  if (publicSignals.length === 0) {
    throw new Error(`${path}: publicSignalsHex must not be empty`);
  }
  return { proofBytes, publicSignals };
}

/**
 * Load a solana-CLI-style JSON keypair: a 64-element array of integers
 * (secret key + public key concatenated, the format produced by
 * `solana-keygen new -o keypair.json`).
 */
export function loadKeypair(path: string): Keypair {
  const raw = readFileSync(path, 'utf-8');
  let parsed: unknown;
  try {
    parsed = JSON.parse(raw);
  } catch (err) {
    throw new Error(`could not parse ${path} as JSON: ${(err as Error).message}`);
  }
  if (!Array.isArray(parsed) || parsed.length !== 64) {
    throw new Error(`${path}: expected a 64-element JSON array (solana-keygen format)`);
  }
  for (const [i, v] of parsed.entries()) {
    if (typeof v !== 'number' || !Number.isInteger(v) || v < 0 || v > 255) {
      throw new Error(`${path}: element ${i} must be a byte (0..255), got ${v}`);
    }
  }
  return Keypair.fromSecretKey(Uint8Array.from(parsed as number[]));
}

function usage(): string {
  return [
    'stargaze-submit-vault-proof',
    '',
    '  --keypair <path>           Solana CLI JSON keypair (required)',
    '  --rpc <url>                RPC endpoint (default https://api.devnet.solana.com)',
    '  --verifier <base58>        Verifier program id (required)',
    '  --provider-id <hex>        32-byte provider id, hex-encoded (required)',
    '  --proof <path>             JSON proof bundle (required, see schema in source)',
    '  --stargaze-program-id <id> Override stargaze_anchor program id',
    `  --compute-units <n>        CU budget (default ${DEFAULT_COMPUTE_UNITS})`,
    `  --commitment <c>           Send commitment (default ${DEFAULT_COMMITMENT})`,
    '  --dry-run                  Build + simulate + print; don\'t send',
  ].join('\n');
}

async function run(argv: string[]): Promise<void> {
  let args: CliArgs;
  try {
    args = parseArgs(argv);
  } catch (err) {
    console.error(`error: ${(err as Error).message}`);
    console.error('');
    console.error(usage());
    process.exit(2);
  }

  const keypair = loadKeypair(args.keypair);
  const providerId = decodeHex(args.providerId, PROVIDER_ID_LEN);
  const bundle = loadProofBundle(args.proof);
  const verifierProgramId = new PublicKey(args.verifier);
  const stargazeProgramId = args.stargazeProgramId
    ? new PublicKey(args.stargazeProgramId)
    : undefined;

  const { instruction, signalsHash, proofRecordPda, vaultConfigPda } =
    buildSubmitVaultProofInstruction({
      submitter: keypair.publicKey,
      verifierProgramId,
      providerId,
      proofBytes: bundle.proofBytes,
      publicSignals: bundle.publicSignals,
      stargazeProgramId,
    });

  console.log(`submitter:        ${keypair.publicKey.toBase58()}`);
  console.log(`vault_config:     ${vaultConfigPda.toBase58()}`);
  console.log(`verifier_program: ${verifierProgramId.toBase58()}`);
  console.log(`proof_record:     ${proofRecordPda.toBase58()}`);
  console.log(`signals_hash:     ${Buffer.from(signalsHash).toString('hex')}`);
  console.log(`public_signals:   ${bundle.publicSignals.length}`);

  const connection = new Connection(args.rpc, args.commitment);
  const tx = new Transaction().add(
    ComputeBudgetProgram.setComputeUnitLimit({ units: args.computeUnits }),
    instruction,
  );
  const { blockhash } = await connection.getLatestBlockhash(args.commitment);
  tx.recentBlockhash = blockhash;
  tx.feePayer = keypair.publicKey;

  if (args.dryRun) {
    tx.sign(keypair);
    const sim = await connection.simulateTransaction(tx);
    console.log(`--- simulate ---`);
    console.log(`err:    ${sim.value.err ? JSON.stringify(sim.value.err) : 'null'}`);
    console.log(`cu:     ${sim.value.unitsConsumed ?? '?'}`);
    for (const line of sim.value.logs ?? []) {
      console.log(`log:    ${line}`);
    }
    if (sim.value.err) {
      process.exit(1);
    }
    return;
  }

  const signature = await connection.sendTransaction(tx, [keypair]);
  console.log(`signature:        ${signature}`);
  const result = await connection.confirmTransaction(
    { signature, blockhash, lastValidBlockHeight: (await connection.getLatestBlockhash(args.commitment)).lastValidBlockHeight },
    args.commitment,
  );
  if (result.value.err) {
    console.error(`confirmation error: ${JSON.stringify(result.value.err)}`);
    process.exit(1);
  }
  console.log(`status:           confirmed`);
}

// Only execute when invoked as a binary, not when imported by tests.
if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  run(process.argv.slice(2)).catch((err) => {
    console.error(`error: ${(err as Error).message}`);
    process.exit(1);
  });
}
