import { createConfig, http } from "wagmi";
import { injected, walletConnect, coinbaseWallet } from "wagmi/connectors";
import { defineChain } from "viem";
import { env } from "../env";

export const tempo = defineChain({
  id: env.tempo.chainId,
  name: "Tempo L1",
  nativeCurrency: { name: "PathUSD", symbol: "PathUSD", decimals: 6 },
  rpcUrls: {
    default: { http: [env.tempo.rpcUrl] },
  },
  blockExplorers: {
    default: { name: "TempoScan", url: "https://tempo.xyz/explorer" },
  },
});

const connectors = [
  injected(),
  ...(env.walletConnectProjectId
    ? [walletConnect({ projectId: env.walletConnectProjectId })]
    : []),
  coinbaseWallet({ appName: "StargazeMPP" }),
];

export const wagmiConfig = createConfig({
  chains: [tempo],
  connectors,
  transports: {
    [tempo.id]: http(),
  },
  ssr: true,
});

declare module "wagmi" {
  interface Register {
    config: typeof wagmiConfig;
  }
}
