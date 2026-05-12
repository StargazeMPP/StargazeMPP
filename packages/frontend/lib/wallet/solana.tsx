"use client";

import { useMemo, type ReactNode } from "react";
import {
  ConnectionProvider,
  WalletProvider,
} from "@solana/wallet-adapter-react";
import {
  PhantomWalletAdapter,
  SolflareWalletAdapter,
} from "@solana/wallet-adapter-wallets";
import { env } from "../env";

interface Props {
  children: ReactNode;
}

/**
 * Solana wallet provider — Phantom, Backpack, Solflare. Reads the RPC
 * endpoint from `lib/env.ts`. Mirrors the connection auto-detection in
 * the frontend build spec §6.2.
 */
export function SolanaWalletProvider({ children }: Props) {
  const wallets = useMemo(
    () => [new PhantomWalletAdapter(), new SolflareWalletAdapter()],
    [],
  );

  return (
    <ConnectionProvider endpoint={env.solana.rpcUrl}>
      <WalletProvider wallets={wallets} autoConnect>
        {children}
      </WalletProvider>
    </ConnectionProvider>
  );
}
