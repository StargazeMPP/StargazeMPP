/**
 * tRPC client placeholder.
 *
 * Once the backend ships `packages/backend` with a `createTRPCRouter` /
 * `AppRouter` export, this module will become:
 *
 * ```ts
 * import { createTRPCReact } from "@trpc/react-query";
 * import type { AppRouter } from "@stargazempp/backend/router";
 * export const trpc = createTRPCReact<AppRouter>();
 * ```
 *
 * Plus a `<TRPCProvider>` wrapper in `app/providers.tsx`. Kept as a
 * placeholder so consumers can import a stable name today and a real
 * client tomorrow without touching call sites.
 */

export type AppRouter = unknown;

export const trpc = {
  __placeholder: true,
} as const;
