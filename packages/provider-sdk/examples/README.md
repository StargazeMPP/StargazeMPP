# Provider SDK examples

End-to-end "monetise an HTTP endpoint" demos for the three most common Node frameworks. Each one is ~40 lines and uses `StargazeMppVerifier` directly — no framework-specific glue.

| File | Framework | Install |
|---|---|---|
| [`express.ts`](express.ts) | Express 4 | `npm i express @types/express tsx` |
| [`hono.ts`](hono.ts) | Hono + node-server | `npm i hono @hono/node-server tsx` |
| [`fastify.ts`](fastify.ts) | Fastify | `npm i fastify tsx` |

Then run any of them with:

```
TEMPO_RPC_URL=https://rpc.tempo.example TEMPO_PATHUSD_ADDRESS=0x... tsx examples/<name>.ts
```

Hit the endpoint with a signed voucher:

```
curl -X POST http://localhost:3000/api/intel \
  -H 'content-type: application/json' \
  -d '{"voucher":{...SignedVoucher...}}'
```

Vouchers are EIP-712 messages signed by the agent's wallet — see `@stargazempp/shared` for `VOUCHER_TYPES` + `buildVoucherDomain`.
