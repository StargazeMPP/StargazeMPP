// Monetise an HTTP endpoint with Fastify + StargazeMppVerifier.
//
// Install: `npm i fastify tsx`
// Run:     `tsx examples/fastify.ts`
//
// The agent POSTs `{ voucher }` where `voucher` is a `SignedVoucher` — a
// Solana Ed25519 signature over the 133-byte StargazeMPP voucher message.
import Fastify from 'fastify';
import { StargazeMppVerifier, type SignedVoucher } from '@stargazempp/provider-sdk';

const verifier = new StargazeMppVerifier({
  solanaRpcUrl: process.env.SOLANA_RPC_URL,
  solanaUsdcMint: process.env.SOLANA_USDC_MINT,
});

const app = Fastify({ logger: true });

app.post<{ Body: { voucher?: SignedVoucher } }>('/api/intel', async (req, reply) => {
  const voucher = req.body?.voucher;
  if (!voucher) return reply.code(400).send({ error: 'missing voucher' });

  try {
    const verified = await verifier.verifyVoucher(voucher);
    return {
      hotTake: 'BTC reclaims $200k by Q3.',
      cumulativeAmount: verified.cumulativeAmount.toString(),
      nonce: verified.nonce.toString(),
    };
  } catch (err) {
    return reply.code(402).send({ error: (err as Error).message });
  }
});

app.listen({ port: 3000 }).then(() => console.log('intel server on :3000'));
