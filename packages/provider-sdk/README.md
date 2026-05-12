# `@stargazempp/provider-sdk`

Drop-in monetisation for any HTTP intelligence endpoint on MPP.

```ts
import { StargazeProvider } from '@stargazempp/provider-sdk';

const provider = new StargazeProvider({
  serviceId: 'my-intelligence',
  category: 'on-chain-analytics',
  pricing: { currency: 'PathUSD', chargeIntent: 0.008, sessionIntent: 0.004 },
  privacy: 'open',
  methods: ['tempo', 'solana'],
  gazeStake: 50,
});

app.post('/api/intel', provider.monetize(async (req, payment) => {
  return { data: await yourLogic(req.body), receipt: payment.receipt };
}));
```

## Ownership split

- **Public surface** — `StargazeProvider`, `monetize`, `vaultMonetize`, config validation, framework adapters (Express / Hono / Fastify). Owned by the external dev.
- **Internal crypto** — `StargazeMppVerifier` (EIP-712 voucher recovery via `viem`, deposit verification via Tempo + Solana RPCs, Groth16 proof generation). Owned by this team. Lives in `src/internal/` and is also exported as a side-door for advanced users.

## What's in `src/internal/` today

- `recoverVoucherSigner(voucher)` — pure EIP-712 ecrecover via viem, sub-10 ms on commodity hardware (the test suite enforces a 10 ms p-mean budget).
- `StargazeMppVerifier` — composes voucher recovery with stubbed deposit verification. `verifyVoucher` is live; `verifyDeposit` throws pending Tempo testnet RPC details (tracked in `BLOCKERS.md`).

## Run the tests

```
npm test --workspace @stargazempp/provider-sdk
```
