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
  solanaRpcUrl: process.env.SOLANA_RPC_URL,
  solanaUsdcMint: process.env.SOLANA_USDC_MINT,
});

const app = new Hono();

app.post('/api/intel', async (c) => {
  const body = (await c.req.json().catch(() => null)) as { voucher?: SignedVoucher } | null;
  const voucher = body?.voucher;
  if (!voucher) return c.json({ error: 'missing voucher' }, 400);

  try {
    const verified = await verifier.verifyVoucher(voucher);
    return c.json({
      hotTake: 'BTC reclaims $200k by Q3.',
      cumulativeAmount: verified.cumulativeAmount.toString(),
      nonce: verified.nonce.toString(),
    });
  } catch (err) {
    return c.json({ error: (err as Error).message }, 402);
  }
});

serve({ fetch: app.fetch, port: 3000 }, ({ port }) => console.log(`intel server on :${port}`));
