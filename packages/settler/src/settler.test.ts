import { describe, expect, it, vi } from 'vitest';
import { maxUint256, type Address, type Hex, type PublicClient, type WalletClient } from 'viem';
import { StaticRateConverter } from './conversion.js';
import { StargazeSettler, type SettlerConfig } from './settler.js';

const ESCROW: Address = '0x1111111111111111111111111111111111111111';
const BURN_CONTROLLER: Address = '0x2222222222222222222222222222222222222222';
const GAZE: Address = '0x3333333333333333333333333333333333333333';
const SETTLER_ADDR: Address = '0x4444444444444444444444444444444444444444';
const SESSION_A: Hex = `0x${'aa'.repeat(32)}`;
const SESSION_B: Hex = `0x${'bb'.repeat(32)}`;

/**
 * Build a settler with mocked viem clients. Returns the settler plus the
 * underlying spies so individual tests can assert on call args.
 *
 * `readContract` returns `maxUint256` for the allowance check by default,
 * so `ensureAllowance` is a no-op — tests that exercise the lazy-approve
 * path override this explicitly.
 */
function buildSettler(overrides: {
  allowance?: bigint;
  rate?: bigint;
} = {}) {
  const writeContract = vi.fn().mockResolvedValue('0xdead' as Hex);
  const readContract = vi.fn().mockResolvedValue(overrides.allowance ?? maxUint256);
  const watchContractEvent = vi.fn().mockReturnValue(() => undefined);

  const walletClient = {
    account: { address: SETTLER_ADDR, type: 'json-rpc' },
    chain: undefined,
    writeContract,
  } as unknown as WalletClient;

  const publicClient = {
    readContract,
    watchContractEvent,
  } as unknown as PublicClient;

  const cfg: SettlerConfig = {
    escrowAddress: ESCROW,
    burnControllerAddress: BURN_CONTROLLER,
    gazeAddress: GAZE,
    walletClient: walletClient as SettlerConfig['walletClient'],
    publicClient,
    converter: new StaticRateConverter(overrides.rate ?? 10n ** 12n),
  };

  return {
    settler: new StargazeSettler(cfg),
    writeContract,
    readContract,
    watchContractEvent,
  };
}

describe('StaticRateConverter', () => {
  it('multiplies PathUSD base units by rate to produce GAZE base units', () => {
    // 1 PathUSD (1e6) at rate 1e12 → 1 GAZE (1e18). Matches the forge test invariant.
    const converter = new StaticRateConverter(10n ** 12n);
    expect(converter.toGaze(10n ** 6n)).toBe(10n ** 18n);
  });

  it('rejects non-positive rates at construction', () => {
    expect(() => new StaticRateConverter(0n)).toThrow(/positive/i);
    expect(() => new StaticRateConverter(-1n)).toThrow(/positive/i);
  });
});

describe('StargazeSettler.handleSettled', () => {
  it('skips zero-fee sessions without calling writeContract', async () => {
    const { settler, writeContract } = buildSettler();

    await settler.handleSettled({ args: { sessionId: SESSION_A, routingFee: 0n } });

    expect(writeContract).not.toHaveBeenCalled();
  });

  it('marks zero-fee sessions as seen so a re-delivery is also a no-op', async () => {
    const { settler, writeContract } = buildSettler();

    await settler.handleSettled({ args: { sessionId: SESSION_A, routingFee: 0n } });
    await settler.handleSettled({ args: { sessionId: SESSION_A, routingFee: 0n } });

    expect(writeContract).not.toHaveBeenCalled();
  });

  it('is idempotent: same sessionId twice → writeContract called once', async () => {
    const { settler, writeContract } = buildSettler();
    const routingFee = 1_000_000n; // 1 PathUSD

    await settler.handleSettled({ args: { sessionId: SESSION_A, routingFee } });
    await settler.handleSettled({ args: { sessionId: SESSION_A, routingFee } });

    expect(writeContract).toHaveBeenCalledTimes(1);
  });

  it('processes distinct sessionIds independently', async () => {
    const { settler, writeContract } = buildSettler();
    const routingFee = 1_000_000n;

    await settler.handleSettled({ args: { sessionId: SESSION_A, routingFee } });
    await settler.handleSettled({ args: { sessionId: SESSION_B, routingFee } });

    expect(writeContract).toHaveBeenCalledTimes(1 /* processRoutingFee for A */ + 1 /* for B */);
  });

  it('calls processRoutingFee with converter-produced gazeFee', async () => {
    const { settler, writeContract } = buildSettler({ rate: 10n ** 12n });
    const routingFee = 1_000_000n; // 1 PathUSD → 1 GAZE

    await settler.handleSettled({ args: { sessionId: SESSION_A, routingFee } });

    expect(writeContract).toHaveBeenCalledTimes(1);
    const call = writeContract.mock.calls[0]?.[0];
    expect(call).toMatchObject({
      address: BURN_CONTROLLER,
      functionName: 'processRoutingFee',
      args: [10n ** 18n],
    });
  });

  it('lazily approves the burn controller for max uint256 when allowance is short', async () => {
    const { settler, writeContract, readContract } = buildSettler({ allowance: 0n });
    const routingFee = 1_000_000n;

    await settler.handleSettled({ args: { sessionId: SESSION_A, routingFee } });

    expect(readContract).toHaveBeenCalledTimes(1);
    expect(readContract.mock.calls[0]?.[0]).toMatchObject({
      address: GAZE,
      functionName: 'allowance',
      args: [SETTLER_ADDR, BURN_CONTROLLER],
    });

    // writeContract is called twice: approve(GAZE, maxUint256), then processRoutingFee.
    expect(writeContract).toHaveBeenCalledTimes(2);
    expect(writeContract.mock.calls[0]?.[0]).toMatchObject({
      address: GAZE,
      functionName: 'approve',
      args: [BURN_CONTROLLER, maxUint256],
    });
    expect(writeContract.mock.calls[1]?.[0]).toMatchObject({
      address: BURN_CONTROLLER,
      functionName: 'processRoutingFee',
      args: [10n ** 18n],
    });
  });

  it('skips the approve when existing allowance already covers the fee', async () => {
    const { settler, writeContract, readContract } = buildSettler({ allowance: maxUint256 });
    const routingFee = 1_000_000n;

    await settler.handleSettled({ args: { sessionId: SESSION_A, routingFee } });

    expect(readContract).toHaveBeenCalledTimes(1);
    expect(writeContract).toHaveBeenCalledTimes(1);
    expect(writeContract.mock.calls[0]?.[0]).toMatchObject({
      functionName: 'processRoutingFee',
    });
  });
});

describe('StargazeSettler.start', () => {
  it('subscribes to SessionSettled on the configured escrow and returns the unwatch handle', () => {
    const { settler, watchContractEvent } = buildSettler();
    const unwatch = vi.fn();
    watchContractEvent.mockReturnValueOnce(unwatch);

    const returned = settler.start();

    expect(watchContractEvent).toHaveBeenCalledTimes(1);
    const params = watchContractEvent.mock.calls[0]?.[0];
    expect(params).toMatchObject({
      address: ESCROW,
      eventName: 'SessionSettled',
    });
    expect(returned).toBe(unwatch);
  });
});
