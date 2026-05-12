// Monetise an HTTP endpoint with Express + StargazeMppVerifier.
//
// Install: `npm i express @types/express tsx`
// Run:     `tsx examples/express.ts`
//
// The agent POSTs `{ voucher }` where `voucher` is a `SignedVoucher` issued
// by their wallet via `signTypedData(VOUCHER_TYPES, ...)`. We recover the
// signer, check it matches the declared `agentWallet`, gate on expiry, and
// only then serve the protected payload.
import express from 'express';
import { StargazeMppVerifier, type SignedVoucher } from '@stargazempp/provider-sdk';

const verifier = new StargazeMppVerifier({
  tempoRpcUrl: process.env.TEMPO_RPC_URL,
  tempoPathUsdAddress: process.env.TEMPO_PATHUSD_ADDRESS as `0x${string}` | undefined,
});

const app = express();
app.use(express.json());

app.post('/api/intel', async (req, res) => {
  const voucher = req.body?.voucher as SignedVoucher | undefined;
  if (!voucher) return res.status(400).json({ error: 'missing voucher' });

  try {
    const { signer, message } = await verifier.verifyVoucher(voucher);
    if (signer.toLowerCase() !== message.agentWallet.toLowerCase()) {
      return res.status(402).json({ error: 'voucher signer mismatch' });
    }
    if (message.expiry < BigInt(Math.floor(Date.now() / 1000))) {
      return res.status(402).json({ error: 'voucher expired' });
    }
    return res.json({
      hotTake: 'BTC reclaims $200k by Q3.',
      cumulativeAmount: message.cumulativeAmount.toString(),
      sessionId: message.sessionId,
    });
  } catch (err) {
    return res.status(402).json({ error: (err as Error).message });
  }
});

app.listen(3000, () => console.log('intel server on :3000'));
