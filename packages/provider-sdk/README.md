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
- **Internal** — `StargazeMppVerifier` (Ed25519 voucher recovery via `tweetnacl`, Solana deposit verification via RPC). Lives in `src/internal/` and is also exported as a side-door for advanced users.

## What's in `src/internal/`

- `recoverVoucherSigner(voucher)` — Ed25519 verify of the 133-byte StargazeMPP voucher via tweetnacl, sub-10 ms on commodity hardware (the test suite enforces a 10 ms p-mean budget).
- `StargazeMppVerifier` — composes voucher recovery with `SolanaDepositVerifier`, which walks SPL Token transfers in a parsed tx.
- `SolanaDepositVerifier` — also exported directly for advanced use.
- `buildSubmitVaultProofInstruction` — builds a `submit_vault_proof` `TransactionInstruction` for the Solana program (see [`bin/submit-vault-proof.ts`](bin/submit-vault-proof.ts) for the matching CLI).
- `parseX402ReceiptIx` — pulls structured fields out of an x402-style USDC `Memo` transfer ix.

## Run the tests

```
npm test --workspace @stargazempp/provider-sdk
```

## Examples

40-line end-to-end demos for the three common Node frameworks live in [`examples/`](examples/):

- [`examples/express.ts`](examples/express.ts)
- [`examples/hono.ts`](examples/hono.ts)
- [`examples/fastify.ts`](examples/fastify.ts)

## CLI: `submit-vault-proof`

[`bin/submit-vault-proof.ts`](bin/submit-vault-proof.ts) is a one-shot
wrapper around `buildSubmitVaultProofInstruction`. Useful for operators
bringing up a verifier on devnet without writing TS glue:

```bash
bun packages/provider-sdk/bin/submit-vault-proof.ts \
    --keypair ~/.config/solana/id.json \
    --rpc https://api.devnet.solana.com \
    --verifier CTC7ehb1sYj7A5EsAd3E6viYdo5bxydzSpccDENbkUmP \
    --provider-id $(cat provider_id.hex) \
    --proof ./aggregate-sum.bundle.json \
    --compute-units 600000
```

The `--proof` argument points at a JSON bundle with the Solana-encoded
proof and public signals:

```json
{
  "proofHex": "<512 hex chars = 256 proof bytes>",
  "publicSignalsHex": ["<64 hex chars>", "<64 hex chars>", "..."]
}
```

Bytes must already be Solana-encoded — BN254 big-endian, c1-first G2,
`pi_a.y` negated. `packages/vault-circuits` ships a one-step emitter:

```bash
cd packages/vault-circuits
npm run prove:aggregate-sum -- --kind bundle \
  --inputs '{"values":[1,2,3,4,5,6,7,8],"claimedSum":36}' \
  > aggregate-sum.bundle.json
```

The bundle that emits is byte-identical to the embedded
`--kind fixture` form used by the on-chain integration tests, so a
proof verified in litesvm and a proof submitted via this CLI share the
same encoding contract.

Add `--dry-run` to simulate the tx + dump program logs without sending.
