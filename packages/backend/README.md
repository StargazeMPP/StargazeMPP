# `@stargazempp/backend`

HTTP API gateway for the StargazeMPP marketplace.

Houses the StargazeIndex Service (provider directory + search), the MPP Session Manager (escrow lifecycle, EIP-712 voucher validation, batch settlement), the Reputation Oracle (elizaOS v2 + Claude API), and the full tRPC route surface (`index.*`, `session.*`, `provider.*`, `vault.*`, `reputation.*`, `gaze.*`).

**Stack.** Express 5 · tRPC 11 · Drizzle ORM · PostgreSQL + TimescaleDB · Redis (Upstash) · viem · `@solana/kit` · `@stargazempp/provider-sdk` · `@stargazempp/shared`.

**Performance targets.** Sub-10-millisecond voucher validation on the hot path (Redis-only, no RPC). 24-hour session TTL. Settlement triggers on close or when the session balance falls below 10% of the original deposit.
