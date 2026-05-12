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

## API surfaces

- **Public** — `StargazeProvider`, `monetize`, `vaultMonetize`, config validation, framework adapters (Express / Hono / Fastify).
- **Internal** — `StargazeMppVerifier` (EIP-712 voucher recovery via `viem`, deposit verification via Tempo + Solana RPCs). Lives in `src/internal/` and is also exported as a side-door for advanced users.

## What's in `src/internal/`

- `recoverVoucherSigner(voucher)` — pure EIP-712 ecrecover via viem, sub-10 ms on commodity hardware (the test suite enforces a 10 ms p-mean budget).
- `StargazeMppVerifier` — composes voucher recovery with Tempo + Solana deposit verification (`TempoDepositVerifier` parses `Transfer` log receipts, `SolanaDepositVerifier` walks SPL Token transfers in a parsed tx).
- `TempoDepositVerifier`, `SolanaDepositVerifier` — also exported directly for advanced use.

## Run the tests

```
npm test --workspace @stargazempp/provider-sdk
```
