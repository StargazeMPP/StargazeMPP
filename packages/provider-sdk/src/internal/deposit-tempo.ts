import {
  createPublicClient,
  decodeEventLog,
  http,
  parseAbiItem,
  type Address,
  type Chain,
  type Hex,
  type Log,
  type PublicClient,
} from 'viem';
import type { VerifiedDeposit } from '@stargazempp/shared';

/**
 * ERC-20 `Transfer(address indexed from, address indexed to, uint256 value)`.
 * Re-derived from the PathUSD token's actual ABI but identical across every
 * standard-compliant token, so we use the abitype-parsed signature directly.
 */
export const TRANSFER_EVENT = parseAbiItem(
  'event Transfer(address indexed from, address indexed to, uint256 value)',
);

export interface TempoDepositVerifierOptions {
  /** Tempo EVM RPC endpoint, e.g. `https://rpc.tempo.xyz` or a testnet variant. */
  rpcUrl: string;
  /** Optional explicit chain definition; defaults to viem auto-detection from the RPC. */
  chain?: Chain;
  /** Address of the PathUSD token contract on the target Tempo network. */
  pathUsdAddress: Address;
  /** Optional pre-built PublicClient — primarily for tests. Wins over `rpcUrl` when set. */
  client?: PublicClient;
}

interface QualifyingTransfer {
  agentWallet: Address;
  amount: bigint;
}

/**
 * Pure log-parsing primitive. Extracted from `TempoDepositVerifier.verify`
 * so unit tests can feed synthetic logs without spinning up an RPC client.
 *
 * Iterates the supplied logs looking for an ERC-20 `Transfer` event emitted
 * by `pathUsdAddress`, whose `to` matches `expectedRecipient` and `value`
 * meets or exceeds `minAmount`. Returns the first qualifying transfer, or
 * `null` if none is present.
 */
export function findQualifyingTransfer(
  logs: readonly Pick<Log, 'address' | 'data' | 'topics'>[],
  pathUsdAddress: Address,
  expectedRecipient: Address,
  minAmount: bigint,
): QualifyingTransfer | null {
  const tokenLower = pathUsdAddress.toLowerCase();
  const recipientLower = expectedRecipient.toLowerCase();
  for (const log of logs) {
    if (log.address.toLowerCase() !== tokenLower) continue;
    let decoded: ReturnType<typeof decodeEventLog<typeof TRANSFER_EVENT_ABI, 'Transfer'>>;
    try {
      decoded = decodeEventLog({
        abi: TRANSFER_EVENT_ABI,
        data: log.data,
        topics: log.topics as [Hex, ...Hex[]],
      });
    } catch {
      continue;
    }
    if (decoded.eventName !== 'Transfer') continue;
    const { from, to, value } = decoded.args;
    if (to.toLowerCase() !== recipientLower) continue;
    if (value < minAmount) continue;
    return { agentWallet: from as Address, amount: value as bigint };
  }
  return null;
}

const TRANSFER_EVENT_ABI = [TRANSFER_EVENT] as const;

/**
 * Verifies a Tempo PathUSD deposit by inspecting an on-chain tx receipt.
 *
 * Flow:
 *   1. Fetch the receipt for `txHash` from the configured RPC.
 *   2. Assert the tx succeeded.
 *   3. Scan the receipt's logs for an ERC-20 `Transfer` emitted by the
 *      PathUSD contract, with `to == expectedRecipient` and
 *      `value >= minAmount`.
 *   4. Return the recovered agent wallet (the `from` field) + amount.
 */
export class TempoDepositVerifier {
  private readonly client: PublicClient;
  private readonly pathUsdAddress: Address;

  constructor(opts: TempoDepositVerifierOptions) {
    this.pathUsdAddress = opts.pathUsdAddress;
    this.client =
      opts.client ??
      (createPublicClient({
        transport: http(opts.rpcUrl),
        chain: opts.chain,
      }) as PublicClient);
  }

  async verify(
    txHash: Hex,
    expectedRecipient: Address,
    minAmount: bigint,
  ): Promise<VerifiedDeposit> {
    const receipt = await this.client.getTransactionReceipt({ hash: txHash });
    if (receipt.status !== 'success') {
      throw new Error(
        `TempoDepositVerifier: transaction ${txHash} did not succeed (status=${receipt.status})`,
      );
    }
    const match = findQualifyingTransfer(
      receipt.logs,
      this.pathUsdAddress,
      expectedRecipient,
      minAmount,
    );
    if (!match) {
      throw new Error(
        `TempoDepositVerifier: no qualifying PathUSD Transfer in ${txHash} (to=${expectedRecipient}, minAmount=${minAmount})`,
      );
    }
    return {
      txHash,
      rail: 'tempo',
      agentWallet: match.agentWallet,
      amount: match.amount,
    };
  }
}
