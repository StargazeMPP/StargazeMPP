// Monetise an HTTP endpoint with Fastify + StargazeMppVerifier.
//
// Install: `npm i fastify tsx`
// Run:     `tsx examples/fastify.ts`
//
// Fastify's typed body schema gives you a route handler whose `req.body`
// is statically typed to `{ voucher: SignedVoucher }` without runtime
// validation overhead.
import Fastify from 'fastify';
import { StargazeMppVerifier, type SignedVoucher } from '@stargazempp/provider-sdk';

const verifier = new StargazeMppVerifier({
  tempoRpcUrl: process.env.TEMPO_RPC_URL,
  tempoPathUsdAddress: process.env.TEMPO_PATHUSD_ADDRESS as `0x${string}` | undefined,
});

const app = Fastify({ logger: true });

app.post<{ Body: { voucher?: SignedVoucher } }>('/api/intel', async (req, reply) => {
  const voucher = req.body?.voucher;
  if (!voucher) return reply.code(400).send({ error: 'missing voucher' });

  try {
    const { signer, message } = await verifier.verifyVoucher(voucher);
    if (signer.toLowerCase() !== message.agentWallet.toLowerCase()) {
      return reply.code(402).send({ error: 'voucher signer mismatch' });
    }
    if (message.expiry < BigInt(Math.floor(Date.now() / 1000))) {
      return reply.code(402).send({ error: 'voucher expired' });
    }
    return {
      hotTake: 'BTC reclaims $200k by Q3.',
      cumulativeAmount: message.cumulativeAmount.toString(),
      sessionId: message.sessionId,
    };
  } catch (err) {
    return reply.code(402).send({ error: (err as Error).message });
  }
});

app.listen({ port: 3000 }).then(() => console.log('intel server on :3000'));
