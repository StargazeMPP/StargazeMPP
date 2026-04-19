export interface DeployedSolanaPrograms {
  /** Base58 program id for `StargazeAnchor`. */
  StargazeAnchor: string;
}

export type SolanaNetwork = 'solana-mainnet' | 'solana-devnet' | 'localnet';

/**
 * Local devnet program ID (matches `target/deploy/stargaze_anchor-keypair.json`).
 * Re-deploys can override; mainnet ID is generated at TGE time.
 */
export const SOLANA_PROGRAMS: Partial<Record<SolanaNetwork, DeployedSolanaPrograms>> = {
  localnet: { StargazeAnchor: 'm6P7kwvXoET9n5B8DFGwwLEozXdv6jBJPdbMiW1TH1R' },
};

export function getSolanaPrograms(network: SolanaNetwork): DeployedSolanaPrograms {
  const programs = SOLANA_PROGRAMS[network];
  if (!programs) {
    throw new Error(
      `StargazeAnchor not yet deployed on ${network}. Update packages/shared/src/solana/programs.ts after the anchor deploy script runs.`,
    );
  }
  return programs;
}

/** Canonical USDC mints — the only mint x402 receipts on Solana should reference. */
export const USDC_MINTS: Record<SolanaNetwork, string> = {
  'solana-mainnet': 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v',
  'solana-devnet': '4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU',
  localnet: '4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU',
};
