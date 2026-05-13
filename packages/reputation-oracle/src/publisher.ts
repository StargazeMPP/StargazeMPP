import { createHash } from 'node:crypto';
import {
  type Connection,
  Keypair,
  PublicKey,
  SystemProgram,
  Transaction,
  TransactionInstruction,
} from '@solana/web3.js';
import type { AggregatedScore } from './vote.js';

/**
 * Anything that can deliver a computed `AggregatedScore` to its sink.
 * Production uses {@link SolanaPublisher}; tests use {@link MemoryPublisher}.
 */
export interface ScorePublisher {
  publish(score: AggregatedScore): Promise<void>;
}

export interface SolanaPublisherConfig {
  /** RPC connection to the target Solana cluster. */
  connection: Connection;
  /** Pays transaction fees. */
  payer: Keypair;
  /** Deployed `StargazeAnchor` program id. */
  programId: PublicKey;
  /**
   * Keypair whose pubkey matches `Config.authority` on-chain. Required to
   * authorise the `set_reputation_score` instruction. May equal `payer`.
   */
  configAuthority: Keypair;
}

const SET_REPUTATION_SCORE_DISCRIMINATOR: Buffer = (() => {
  const h = createHash('sha256');
  h.update('global:set_reputation_score');
  return h.digest().subarray(0, 8);
})();

function providerIdToBytes(providerId: `0x${string}`): Buffer {
  const hex = providerId.startsWith('0x') ? providerId.slice(2) : providerId;
  if (hex.length !== 64) {
    throw new Error(
      `SolanaPublisher: providerId must encode 32 bytes, got ${hex.length / 2}`,
    );
  }
  return Buffer.from(hex, 'hex');
}

function buildSetReputationScoreData(providerIdBytes: Buffer, score: number): Buffer {
  // Layout: discriminator(8) | provider_id([u8;32]) | new_score(u16 LE)
  const data = Buffer.alloc(8 + 32 + 2);
  SET_REPUTATION_SCORE_DISCRIMINATOR.copy(data, 0);
  providerIdBytes.copy(data, 8);
  data.writeUInt16LE(score, 8 + 32);
  return data;
}

/**
 * Publishes scores on-chain by calling the `set_reputation_score` Anchor
 * instruction on `StargazeAnchor`. The instruction is authority-gated:
 * `configAuthority.publicKey` MUST equal `Config.authority` recorded
 * during `initialize`.
 */
export class SolanaPublisher implements ScorePublisher {
  constructor(private readonly cfg: SolanaPublisherConfig) {}

  async publish(s: AggregatedScore): Promise<void> {
    const { connection, payer, programId, configAuthority } = this.cfg;
    const providerIdBytes = providerIdToBytes(s.providerId);

    const [configPda] = PublicKey.findProgramAddressSync(
      [Buffer.from('config')],
      programId,
    );
    const [providerPda] = PublicKey.findProgramAddressSync(
      [Buffer.from('provider'), providerIdBytes],
      programId,
    );

    const ix = new TransactionInstruction({
      programId,
      keys: [
        { pubkey: configAuthority.publicKey, isSigner: true, isWritable: false },
        { pubkey: configPda, isSigner: false, isWritable: false },
        { pubkey: providerPda, isSigner: false, isWritable: true },
      ],
      data: buildSetReputationScoreData(providerIdBytes, s.score),
    });

    const tx = new Transaction().add(ix);
    tx.feePayer = payer.publicKey;
    const { blockhash, lastValidBlockHeight } = await connection.getLatestBlockhash();
    tx.recentBlockhash = blockhash;
    tx.lastValidBlockHeight = lastValidBlockHeight;

    const signers = [payer];
    if (!configAuthority.publicKey.equals(payer.publicKey)) {
      signers.push(configAuthority);
    }
    tx.sign(...signers);
    const raw = tx.serialize();
    await connection.sendRawTransaction(raw, {
      preflightCommitment: 'confirmed',
    });
  }
}

/**
 * In-memory publisher used by tests. Records every published score for
 * later assertion. No side effects beyond the in-process array.
 */
export class MemoryPublisher implements ScorePublisher {
  public readonly published: AggregatedScore[] = [];

  async publish(s: AggregatedScore): Promise<void> {
    this.published.push(s);
  }
}

// Re-exported because the construction of a real Solana publisher commonly
// needs a System Program reference in surrounding wiring; downstream
// callers don't have to depend on `@solana/web3.js` just to grab it.
export { SystemProgram };
