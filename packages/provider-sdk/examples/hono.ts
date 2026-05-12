// Monetise an HTTP endpoint with Hono + StargazeMppVerifier.
//
// Install: `npm i hono @hono/node-server tsx`
// Run:     `tsx examples/hono.ts`
//
// Same voucher-gating flow as the Express example; Hono just lets you
// deploy the same handler on Bun, Deno, Cloudflare Workers, etc.
import { Hono } from 'hono';
import { serve } from '@hono/node-server';
import { StargazeMppVerifier, type SignedVoucher } from '@stargazempp/provider-sdk';

const verifier = new StargazeMppVerifier({
  tempoRpcUrl: process.env.TEMPO_RPC_URL,
  tempoPathUsdAddress: process.env.TEMPO_PATHUSD_ADDRESS as `0x${string}` | undefined,
});

const app = new Hono();

app.post('/api/intel', async (c) => {
  const body = (await c.req.json().catch(() => null)) as { voucher?: SignedVoucher } | null;
  const voucher = body?.voucher;
  if (!voucher) return c.json({ error: 'missing voucher' }, 400);

  try {
    const { signer, message } = await verifier.verifyVoucher(voucher);
    if (signer.toLowerCase() !== message.agentWallet.toLowerCase()) {
      return c.json({ error: 'voucher signer mismatch' }, 402);
    }
    if (message.expiry < BigInt(Math.floor(Date.now() / 1000))) {
      return c.json({ error: 'voucher expired' }, 402);
    }
    return c.json({
      hotTake: 'BTC reclaims $200k by Q3.',
      cumulativeAmount: message.cumulativeAmount.toString(),
      sessionId: message.sessionId,
    });
  } catch (err) {
    return c.json({ error: (err as Error).message }, 402);
  }
});

serve({ fetch: app.fetch, port: 3000 }, ({ port }) => console.log(`intel server on :${port}`));
