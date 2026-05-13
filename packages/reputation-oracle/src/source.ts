import type { Pool, QueryResultRow } from 'pg';
import bs58 from 'bs58';
import type { Vote } from './vote.js';

/**
 * Abstract source of `ReputationVoted` events. Implementations supply a
 * slot-ordered stream so the oracle's cursor can advance monotonically.
 */
export interface VoteSource {
  /**
   * Load every vote with `slot >= sinceSlot`, ordered by slot ascending.
   * Implementations should be idempotent — replaying the same slot range
   * must yield the same vote set (the indexer's primary key is `(slot,
   * signature)`).
   */
  loadSince(sinceSlot: bigint): Promise<Vote[]>;
}

/**
 * Postgres-backed `VoteSource` reading the indexer's `reputation_voted`
 * projection (see
 * `packages/indexer/migrations/20260513000000_init.sql`).
 *
 * Schema:
 *   - `provider_id BYTEA` — 32-byte providerId, re-encoded as 0x-hex.
 *   - `voter       BYTEA` — 32-byte Solana pubkey, re-encoded as base58.
 *   - `accurate    BOOL`
 *   - `slot        BIGINT`
 *   - `signature   TEXT`
 *   - `created_at  TIMESTAMPTZ`
 */
export class PostgresVoteSource implements VoteSource {
  constructor(private readonly pool: Pool) {}

  async loadSince(sinceSlot: bigint): Promise<Vote[]> {
    const result = await this.pool.query<ReputationVotedRow>(
      `SELECT provider_id, voter, accurate, slot, signature, created_at
       FROM reputation_voted
       WHERE slot >= $1
       ORDER BY slot ASC, id ASC`,
      [sinceSlot.toString()],
    );

    return result.rows.map((row) => ({
      providerId: bytesToHex(row.provider_id),
      voter: bs58.encode(toBuffer(row.voter)),
      accurate: row.accurate,
      slot: BigInt(row.slot),
      signature: row.signature || undefined,
      votedAt: row.created_at,
    }));
  }
}

interface ReputationVotedRow extends QueryResultRow {
  provider_id: Buffer | Uint8Array;
  voter: Buffer | Uint8Array;
  accurate: boolean;
  // `pg` returns BIGINT as string by default to avoid precision loss.
  slot: string;
  signature: string;
  created_at: Date;
}

function toBuffer(value: Buffer | Uint8Array): Uint8Array {
  return value instanceof Uint8Array ? value : new Uint8Array(value);
}

function bytesToHex(value: Buffer | Uint8Array): `0x${string}` {
  const bytes = toBuffer(value);
  let hex = '0x';
  for (const b of bytes) hex += b.toString(16).padStart(2, '0');
  return hex as `0x${string}`;
}
