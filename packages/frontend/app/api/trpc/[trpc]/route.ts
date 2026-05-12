/**
 * tRPC API handler placeholder.
 *
 * Until the backend ships an `AppRouter`, this route responds with a
 * 501 so clients fail fast instead of hanging. Once `packages/backend`
 * exports a router, replace the body with:
 *
 * ```ts
 * import { fetchRequestHandler } from "@trpc/server/adapters/fetch";
 * import { appRouter } from "@stargazempp/backend/router";
 * import { createContext } from "@stargazempp/backend/context";
 *
 * const handler = (req: Request) =>
 *   fetchRequestHandler({
 *     endpoint: "/api/trpc",
 *     req,
 *     router: appRouter,
 *     createContext,
 *   });
 *
 * export { handler as GET, handler as POST };
 * ```
 */

export const dynamic = "force-dynamic";

function notImplemented() {
  return new Response(
    JSON.stringify({
      code: "tRPC_not_wired",
      message:
        "The backend tRPC router has not been deployed yet. Use the REST endpoints under /v1/* in the meantime.",
    }),
    {
      status: 501,
      headers: { "content-type": "application/json" },
    },
  );
}

export async function GET() {
  return notImplemented();
}

export async function POST() {
  return notImplemented();
}
