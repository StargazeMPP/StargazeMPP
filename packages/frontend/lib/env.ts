import { z } from "zod";

/**
 * Runtime + build-time environment contract for the StargazeMPP front-end.
 *
 * The Zod schema fails fast at boot if a required variable is missing,
 * which lets the rest of the app treat `env.*` accesses as definitely
 * defined. `NEXT_PUBLIC_*` values are inlined into client bundles by
 * Next.js; everything else is server-only.
 *
 * Per the frontend build spec (§3), these names are stable — do not
 * rename without coordinating across backend + infra.
 */

const publicEnv = z.object({
  NEXT_PUBLIC_API_URL: z.string().url().default("http://localhost:8080"),

  // Tempo EVM rail
  NEXT_PUBLIC_TEMPO_CHAIN_ID: z.coerce.number().int().positive().default(1789),
  NEXT_PUBLIC_TEMPO_RPC_URL: z
    .string()
    .url()
    .default("https://rpc.tempo.xyz"),

  // Solana rail
  NEXT_PUBLIC_SOLANA_RPC_URL: z
    .string()
    .url()
    .default("https://api.mainnet-beta.solana.com"),

  // Deployed contract addresses (Tempo EVM)
  NEXT_PUBLIC_SESSION_ESCROW_ADDR: z
    .string()
    .startsWith("0x")
    .default("0x0000000000000000000000000000000000000000"),
  NEXT_PUBLIC_PROVIDER_REGISTRY_ADDR: z
    .string()
    .startsWith("0x")
    .default("0x0000000000000000000000000000000000000000"),
  NEXT_PUBLIC_GAZE_TOKEN_ADDR: z
    .string()
    .startsWith("0x")
    .default("0x0000000000000000000000000000000000000000"),
  NEXT_PUBLIC_REPUTATION_ORACLE_ADDR: z
    .string()
    .startsWith("0x")
    .default("0x0000000000000000000000000000000000000000"),

  // Third-party
  NEXT_PUBLIC_WALLETCONNECT_PROJECT_ID: z.string().default(""),
  NEXT_PUBLIC_POSTHOG_KEY: z.string().default(""),
});

const serverEnv = z.object({
  SENTRY_DSN: z.string().default(""),
  INTERNAL_API_TOKEN: z.string().default(""),
});

const parsedPublic = publicEnv.parse({
  NEXT_PUBLIC_API_URL: process.env.NEXT_PUBLIC_API_URL,
  NEXT_PUBLIC_TEMPO_CHAIN_ID: process.env.NEXT_PUBLIC_TEMPO_CHAIN_ID,
  NEXT_PUBLIC_TEMPO_RPC_URL: process.env.NEXT_PUBLIC_TEMPO_RPC_URL,
  NEXT_PUBLIC_SOLANA_RPC_URL: process.env.NEXT_PUBLIC_SOLANA_RPC_URL,
  NEXT_PUBLIC_SESSION_ESCROW_ADDR: process.env.NEXT_PUBLIC_SESSION_ESCROW_ADDR,
  NEXT_PUBLIC_PROVIDER_REGISTRY_ADDR: process.env.NEXT_PUBLIC_PROVIDER_REGISTRY_ADDR,
  NEXT_PUBLIC_GAZE_TOKEN_ADDR: process.env.NEXT_PUBLIC_GAZE_TOKEN_ADDR,
  NEXT_PUBLIC_REPUTATION_ORACLE_ADDR: process.env.NEXT_PUBLIC_REPUTATION_ORACLE_ADDR,
  NEXT_PUBLIC_WALLETCONNECT_PROJECT_ID: process.env.NEXT_PUBLIC_WALLETCONNECT_PROJECT_ID,
  NEXT_PUBLIC_POSTHOG_KEY: process.env.NEXT_PUBLIC_POSTHOG_KEY,
});

const parsedServer =
  typeof window === "undefined"
    ? serverEnv.parse({
        SENTRY_DSN: process.env.SENTRY_DSN,
        INTERNAL_API_TOKEN: process.env.INTERNAL_API_TOKEN,
      })
    : ({ SENTRY_DSN: "", INTERNAL_API_TOKEN: "" } as z.infer<typeof serverEnv>);

export const env = {
  apiUrl: parsedPublic.NEXT_PUBLIC_API_URL,
  tempo: {
    chainId: parsedPublic.NEXT_PUBLIC_TEMPO_CHAIN_ID,
    rpcUrl: parsedPublic.NEXT_PUBLIC_TEMPO_RPC_URL,
  },
  solana: {
    rpcUrl: parsedPublic.NEXT_PUBLIC_SOLANA_RPC_URL,
  },
  contracts: {
    sessionEscrow: parsedPublic.NEXT_PUBLIC_SESSION_ESCROW_ADDR as `0x${string}`,
    providerRegistry: parsedPublic.NEXT_PUBLIC_PROVIDER_REGISTRY_ADDR as `0x${string}`,
    gazeToken: parsedPublic.NEXT_PUBLIC_GAZE_TOKEN_ADDR as `0x${string}`,
    reputationOracle: parsedPublic.NEXT_PUBLIC_REPUTATION_ORACLE_ADDR as `0x${string}`,
  },
  walletConnectProjectId: parsedPublic.NEXT_PUBLIC_WALLETCONNECT_PROJECT_ID,
  posthogKey: parsedPublic.NEXT_PUBLIC_POSTHOG_KEY,
  sentryDsn: parsedServer.SENTRY_DSN,
  internalApiToken: parsedServer.INTERNAL_API_TOKEN,
} as const;

export type Env = typeof env;
