import { describe, it, expect } from 'vitest';
import {
  encodeEventTopics,
  encodeAbiParameters,
  parseAbiParameters,
  type Address,
  type Hex,
} from 'viem';
import { findQualifyingTransfer, TRANSFER_EVENT } from './deposit-tempo.js';

const PATHUSD: Address = '0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa' as Address;
const ESCROW: Address = '0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb' as Address;
const ATTACKER: Address = '0xcccccccccccccccccccccccccccccccccccccccc' as Address;
const AGENT: Address = '0xdddddddddddddddddddddddddddddddddddddddd' as Address;

function transferLog(from: Address, to: Address, value: bigint, tokenAddress: Address = PATHUSD) {
  const topics = encodeEventTopics({
    abi: [TRANSFER_EVENT],
    eventName: 'Transfer',
    args: { from, to },
  });
  const data: Hex = encodeAbiParameters(parseAbiParameters('uint256'), [value]);
  return { address: tokenAddress, topics, data };
}

describe('findQualifyingTransfer', () => {
  it('returns the matching transfer when amount meets the floor', () => {
    const logs = [transferLog(AGENT, ESCROW, 100_000_000n)];
    const result = findQualifyingTransfer(logs, PATHUSD, ESCROW, 50_000_000n);
    expect(result).not.toBeNull();
    expect(result?.agentWallet.toLowerCase()).toBe(AGENT.toLowerCase());
    expect(result?.amount).toBe(100_000_000n);
  });

  it('rejects transfers whose recipient does not match the escrow', () => {
    const logs = [transferLog(AGENT, ATTACKER, 100_000_000n)];
    expect(findQualifyingTransfer(logs, PATHUSD, ESCROW, 1n)).toBeNull();
  });

  it('rejects transfers below minAmount', () => {
    const logs = [transferLog(AGENT, ESCROW, 49_999_999n)];
    expect(findQualifyingTransfer(logs, PATHUSD, ESCROW, 50_000_000n)).toBeNull();
  });

  it('ignores transfers emitted by other token contracts', () => {
    const otherToken: Address = '0xDecoy000000000000000000000000000000000000' as Address;
    const logs = [transferLog(AGENT, ESCROW, 100_000_000n, otherToken)];
    expect(findQualifyingTransfer(logs, PATHUSD, ESCROW, 1n)).toBeNull();
  });

  it('skips non-Transfer logs without throwing', () => {
    const logs = [
      // A garbage log with junk topics — should be tolerated.
      { address: PATHUSD, topics: ['0xdead'], data: '0x00' as Hex },
      transferLog(AGENT, ESCROW, 100_000_000n),
    ];
    const result = findQualifyingTransfer(
      logs as unknown as Parameters<typeof findQualifyingTransfer>[0],
      PATHUSD,
      ESCROW,
      1n,
    );
    expect(result?.amount).toBe(100_000_000n);
  });

  it('returns the first matching transfer when multiple qualify', () => {
    const logs = [
      transferLog(AGENT, ESCROW, 100_000_000n),
      transferLog(ATTACKER, ESCROW, 200_000_000n),
    ];
    const result = findQualifyingTransfer(logs, PATHUSD, ESCROW, 1n);
    expect(result?.agentWallet.toLowerCase()).toBe(AGENT.toLowerCase());
  });
});
