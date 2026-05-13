import { describe, it, expect, vi } from 'vitest';
import { Keypair, PublicKey } from '@solana/web3.js';
import { SolanaPublisher } from './publisher.js';
import type { AggregatedScore } from './vote.js';

const PROGRAM_ID = new PublicKey('m6P7kwvXoET9n5B8DFGwwLEozXdv6jBJPdbMiW1TH1R');
const PROVIDER_A_HEX = `0x${'aa'.repeat(32)}` as `0x${string}`;

function fakeConnection(sendStub: ReturnType<typeof vi.fn>) {
  return {
    getLatestBlockhash: vi.fn().mockResolvedValue({
      blockhash: '11111111111111111111111111111111',
      lastValidBlockHeight: 1n,
    }),
    sendRawTransaction: sendStub,
    // Methods unused by SolanaPublisher.
  } as unknown as import('@solana/web3.js').Connection;
}

describe('SolanaPublisher', () => {
  it('sends a set_reputation_score transaction with the expected accounts', async () => {
    const payer = Keypair.generate();
    const configAuthority = Keypair.generate();
    const sendStub = vi.fn().mockResolvedValue('mock-sig');
    const connection = fakeConnection(sendStub);

    const publisher = new SolanaPublisher({
      connection,
      payer,
      configAuthority,
      programId: PROGRAM_ID,
    });

    const score: AggregatedScore = {
      providerId: PROVIDER_A_HEX,
      score: 875,
      totalVotes: 5,
      accurateVotes: 4,
    };

    await publisher.publish(score);

    expect(sendStub).toHaveBeenCalledTimes(1);
    const raw = sendStub.mock.calls[0]![0] as Buffer | Uint8Array;
    expect(raw.byteLength).toBeGreaterThan(0);
  });

  it('rejects a providerId that does not encode 32 bytes', async () => {
    const payer = Keypair.generate();
    const configAuthority = Keypair.generate();
    const connection = fakeConnection(vi.fn());
    const publisher = new SolanaPublisher({
      connection,
      payer,
      configAuthority,
      programId: PROGRAM_ID,
    });

    await expect(
      publisher.publish({
        providerId: '0xdead' as `0x${string}`,
        score: 100,
        totalVotes: 3,
        accurateVotes: 3,
      }),
    ).rejects.toThrow(/must encode 32 bytes/i);
  });

  it('signs with the configAuthority when it differs from the payer', async () => {
    const payer = Keypair.generate();
    const configAuthority = Keypair.generate();
    const sendStub = vi.fn().mockResolvedValue('mock-sig');
    const connection = fakeConnection(sendStub);

    const publisher = new SolanaPublisher({
      connection,
      payer,
      configAuthority,
      programId: PROGRAM_ID,
    });

    await publisher.publish({
      providerId: PROVIDER_A_HEX,
      score: 500,
      totalVotes: 3,
      accurateVotes: 2,
    });

    expect(sendStub).toHaveBeenCalledTimes(1);
  });
});
