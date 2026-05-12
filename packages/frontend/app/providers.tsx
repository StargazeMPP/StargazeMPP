"use client";

import { useState, type ReactNode } from "react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { WagmiProvider } from "wagmi";
import { ThemeProvider } from "next-themes";
import { wagmiConfig } from "@/lib/wallet/wagmi";
import { SolanaWalletProvider } from "@/lib/wallet/solana";

interface Props {
  children: ReactNode;
}

/**
 * Top-level React context providers. Must be mounted from the root
 * layout. Order matters — wagmi outside Solana lets EVM-only pages
 * skip the Solana RPC; theme outermost so SSR-rendered HTML carries the
 * `data-theme` attribute on first paint.
 */
export default function Providers({ children }: Props) {
  const [queryClient] = useState(
    () =>
      new QueryClient({
        defaultOptions: {
          queries: {
            staleTime: 30_000,
            refetchOnWindowFocus: false,
          },
        },
      }),
  );

  return (
    <ThemeProvider attribute="data-theme" defaultTheme="dark" enableSystem={false}>
      <QueryClientProvider client={queryClient}>
        <WagmiProvider config={wagmiConfig}>
          <SolanaWalletProvider>{children}</SolanaWalletProvider>
        </WagmiProvider>
      </QueryClientProvider>
    </ThemeProvider>
  );
}
