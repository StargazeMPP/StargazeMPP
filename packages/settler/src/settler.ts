import type {
  Account,
  Address,
  Chain,
  Hex,
  PublicClient,
  Transport,
  WalletClient,
} from 'viem';
import { maxUint256, parseAbi } from 'viem';
import type { PathUsdToGazeConverter } from './conversion.js';

/**
 * Inlined minimal ABIs. We only need the `SessionSettled` event signature on
 * the escrow side and the `processRoutingFee` + ERC-20 allowance/approve calls
 * on the burn-controller / GAZE side. Inlining (rather than importing the
 * widened JSON ABIs from `@stargazempp/shared`) keeps viem's generic type
 * inference happy — `watchContractEvent` infers `onLogs`'s argument types
 * from a narrow `const`-tagged ABI literal.
 *
 * Keep these in sync with `packages/contracts-evm/src/{StargazeEscrow,BurnController}.sol`.
 */
export const STARGAZE_ESCROW_SESSION_SETTLED_ABI = parseAbi([
  'event SessionSettled(bytes32 indexed sessionId, uint256 totalToProviders, uint256 routingFee, uint256 refundToAgent)',
]);

export const BURN_CONTROLLER_PROCESS_FEE_ABI = parseAbi([
  'function processRoutingFee(uint256 feeAmount)',
]);

export const ERC20_ALLOWANCE_ABI = parseAbi([
  'function allowance(address owner, address spender) view returns (uint256)',
  'function approve(address spender, uint256 value) returns (bool)',
]);

export interface SettlerConfig {
  /** Deployed `StargazeEscrow` address. The bot subscribes to its `SessionSettled` event. */
  escrowAddress: Address;
  /** Deployed `BurnController` address. The bot calls `processRoutingFee` on this contract. */
  burnControllerAddress: Address;
  /** Deployed `GAZEToken` address. The bot manages an allowance from itself to the burn controller. */
  gazeAddress: Address;
  /** Wallet client whose account is the settler EOA — must hold ROUTER_ROLE on the burn controller and have $GAZE. */
  walletClient: WalletClient<Transport, Chain | undefined, Account>;
  /** Public client used for subscriptions and read calls (allowance check). */
  publicClient: PublicClient;
  /** PathUSD → GAZE conversion strategy. v1 is `StaticRateConverter`. */
  converter: PathUsdToGazeConverter;
  /** Optional block to start watching from. If unset, viem watches from "latest". */
  fromBlock?: bigint;
}

/**
 * Subset of a `SessionSettled` log that the bot actually uses. Decoupled
 * from viem's `Log<...>` type so unit tests can build synthetic inputs
 * without re-typing every field.
 */
export interface SessionSettledLogArgs {
  sessionId: Hex;
  routingFee: bigint;
}

/**
 * Off-chain settler bot.
 *
 * Loop:
 *   1. `StargazeEscrow.settle` emits `SessionSettled(sessionId, _, routingFee, _)`
 *      and transfers `routingFee` PathUSD to the `routingFeeSink` (the bot's EOA).
 *   2. The bot is pre-funded with $GAZE and converts the fee off-chain (v1: static rate).
 *   3. The bot calls `BurnController.processRoutingFee(gazeFee)`, which burns 50%
 *      and forwards 50% to the staker pool — both legs pull from `msg.sender`.
 *
 * Idempotency: an in-memory `Set<Hex>` of processed sessionIds prevents a
 * duplicate fire-and-forget call if viem re-delivers a log (which it can,
 * around reorgs). Production must replace this with durable storage.
 */
export class StargazeSettler {
  private readonly cfg: SettlerConfig;
  private readonly seen = new Set<Hex>();

  constructor(cfg: SettlerConfig) {
    this.cfg = cfg;
  }

  /**
   * Start watching the escrow for `SessionSettled`. Returns an `unwatch`
   * function — call it to stop the bot.
   *
   * `onLogs` runs the full handler chain serially per log; errors are
   * surfaced via `onError` (logged, then re-raised would crash the watcher,
   * so we just log here and let the caller decide via the returned handle).
   */
  start(): () => void {
    return this.cfg.publicClient.watchContractEvent({
      address: this.cfg.escrowAddress,
      abi: STARGAZE_ESCROW_SESSION_SETTLED_ABI,
      eventName: 'SessionSettled',
      fromBlock: this.cfg.fromBlock,
      onLogs: async (logs) => {
        for (const log of logs) {
          const { sessionId, routingFee } = log.args;
          if (sessionId === undefined || routingFee === undefined) {
            // `strict: false` (default) means args can be partial when the
            // log fails decoding — skip those rather than crash the watcher.
            continue;
          }
          try {
            await this.handleSettled({ args: { sessionId, routingFee } });
          } catch (err) {
            // Surface but don't crash — a single failed session shouldn't
            // take down the bot. Production should hook this into metrics
            // / alerting and a durable retry queue.
            console.error(
              `[StargazeSettler] handleSettled failed for sessionId=${sessionId}:`,
              err,
            );
          }
        }
      },
      onError: (err) => {
        console.error('[StargazeSettler] watchContractEvent error:', err);
      },
    });
  }

  /**
   * Public for testability — the inner per-log handler. The `start()`
   * onLogs callback feeds normalised logs here, and tests can call this
   * directly with synthetic args.
   *
   * Skips:
   *   - sessionIds already in the in-memory seen-set (idempotent)
   *   - `routingFee == 0n` (would revert with `ZeroAmount` on-chain)
   */
  async handleSettled(log: { args: SessionSettledLogArgs }): Promise<void> {
    const { sessionId, routingFee } = log.args;

    if (this.seen.has(sessionId)) {
      return;
    }
    if (routingFee === 0n) {
      // `BurnController.processRoutingFee` reverts on zero — skip cleanly.
      // Mark seen so a re-delivery of the same log doesn't re-enter.
      this.seen.add(sessionId);
      return;
    }

    const gazeFee = this.cfg.converter.toGaze(routingFee);
    if (gazeFee === 0n) {
      // Defensive: if the converter produced zero (e.g. a misconfigured rate),
      // the on-chain call would still revert. Skip + mark seen.
      this.seen.add(sessionId);
      return;
    }

    await this.ensureAllowance(gazeFee);

    await this.cfg.walletClient.writeContract({
      address: this.cfg.burnControllerAddress,
      abi: BURN_CONTROLLER_PROCESS_FEE_ABI,
      functionName: 'processRoutingFee',
      args: [gazeFee],
      account: this.cfg.walletClient.account,
      chain: this.cfg.walletClient.chain,
    });

    this.seen.add(sessionId);
  }

  /**
   * Lazy approve: if the current GAZE allowance from the settler to the
   * burn controller is below `required`, top it up to `maxUint256`. Single
   * `approve` per bot lifetime in the common case.
   */
  private async ensureAllowance(required: bigint): Promise<void> {
    const owner = this.cfg.walletClient.account.address;
    const current = (await this.cfg.publicClient.readContract({
      address: this.cfg.gazeAddress,
      abi: ERC20_ALLOWANCE_ABI,
      functionName: 'allowance',
      args: [owner, this.cfg.burnControllerAddress],
    })) as bigint;

    if (current >= required) return;

    await this.cfg.walletClient.writeContract({
      address: this.cfg.gazeAddress,
      abi: ERC20_ALLOWANCE_ABI,
      functionName: 'approve',
      args: [this.cfg.burnControllerAddress, maxUint256],
      account: this.cfg.walletClient.account,
      chain: this.cfg.walletClient.chain,
    });
  }
}
