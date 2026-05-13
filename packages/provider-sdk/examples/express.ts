// Monetise an HTTP endpoint with Express + StargazeMppVerifier.
//
// Install: `npm i express @types/express tsx`
// Run:     `tsx examples/express.ts`
//
// The agent POSTs `{ voucher }` where `voucher` is a `SignedVoucher` — a
// Solana Ed25519 signature over the 133-byte StargazeMPP voucher message.
// We verify the signature and amount, then serve the protected payload.
import express from 'express';
import { StargazeMppVerifier, type SignedVoucher } from '@stargazempp/provider-sdk';

const verifier = new StargazeMppVerifier({
  solanaRpcUrl: process.env.SOLANA_RPC_URL,
  solanaUsdcMint: process.env.SOLANA_USDC_MINT,
});

const app = express();
app.use(express.json());

app.post('/api/intel', async (req, res) => {
  const voucher = req.body?.voucher as SignedVoucher | undefined;
  if (!voucher) return res.status(400).json({ error: 'missing voucher' });

  try {
    const verified = await verifier.verifyVoucher(voucher);
    return res.json({
      hotTake: 'BTC reclaims $200k by Q3.',
      cumulativeAmount: verified.cumulativeAmount.toString(),
      nonce: verified.nonce.toString(),
    });
  } catch (err) {
    return res.status(402).json({ error: (err as Error).message });
  }
});

app.listen(3000, () => console.log('intel server on :3000'));
