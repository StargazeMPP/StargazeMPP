# @stargazempp/settler

Off-chain bot that closes the StargazeMPP routing-fee loop. It watches
`StargazeEscrow.SessionSettled`, converts the PathUSD routing fee to $GAZE,
and calls `BurnController.processRoutingFee(gazeFee)` so half is burned and
half is forwarded to the staker pool. This is a **reference implementation**
intended to anchor the integration; see the limitations section below before
running it against real funds.

## How it works

1. `StargazeEscrow.settle` emits `SessionSettled(sessionId, totalToProviders, routingFee, refundToAgent)`
   and transfers `routingFee` PathUSD to the configured `routingFeeSink`
   (which must be set to this bot's EOA at deploy time).
2. The bot reads `routingFee` directly off the log, converts it to GAZE
   base units via a `PathUsdToGazeConverter` (v1: `StaticRateConverter`),
   ensures its allowance to `BurnController` is sufficient (lazy approve to
   `maxUint256`), and submits `processRoutingFee(gazeFee)`.
3. `BurnController` burns 50% of `gazeFee` from `msg.sender` and forwards
   the other 50% to the staker pool. Both legs pull from the settler EOA,
   so the bot must hold GAZE.

## Environment variables (reference wiring)

The package exports the `StargazeSettler` class; bring your own entrypoint
that reads these (or equivalent) values and wires the viem clients:

- `ESCROW_ADDRESS` — deployed `StargazeEscrow`.
- `BURN_CONTROLLER_ADDRESS` — deployed `BurnController`.
- `GAZE_ADDRESS` — deployed `GAZEToken`.
- `RPC_URL` — Tempo (EVM) JSON-RPC endpoint, used for both `publicClient` and `walletClient`.
- `SETTLER_PRIVATE_KEY` — EOA that holds GAZE, has `ROUTER_ROLE` on the
  `BurnController`, and is configured as the escrow's `routingFeeSink`.
- `PATH_USD_TO_GAZE_RATE` — decimal string parseable as `bigint`, in
  "GAZE base units per PathUSD base unit". Example: `"1000000000000"`
  pegs 1 PathUSD (1e6) to 1 GAZE (1e18).
- `START_BLOCK` (optional) — block to begin watching from. Defaults to "latest".

## Pre-flight checklist (admin, not bot)

These deploy-time grants must already be in place before the bot is useful:

- `StargazeEscrow.grantRole(ROUTER_ROLE, <payment-router>)` for the contract that calls `settle`.
- `StargazeEscrow.setRoutingFeeSink(<settler EOA>)` so the 2% PathUSD fee lands at the bot.
- `BurnController.grantRole(ROUTER_ROLE, <settler EOA>)` so the bot can call `processRoutingFee`.
- `BurnController.setStakerPool(<staker pool>)` so the 50% staker leg has a destination.
- Settler EOA holds enough GAZE to cover anticipated routing-fee volume.

On first run the bot will `approve(BurnController, maxUint256)` itself.

## Known limitations (v1)

- **In-memory seen-set.** Restarting the bot loses idempotency state. If the
  RPC re-delivers an old log after a restart, the bot may try to process the
  same session twice. Production must back the seen-set with durable
  storage (Postgres/SQLite) and acknowledge logs against a write-ahead log.
- **No DEX swap.** The bot assumes the settler EOA is pre-funded with GAZE.
  PathUSD that lands at the sink stays there until something else moves it.
  If the settler runs out of GAZE, `processRoutingFee` will revert and the
  fee will accumulate in PathUSD at the sink address.
- **Crash window between `settle` and `processRoutingFee`.** Settlement
  lands PathUSD at the sink (step a). If the bot crashes before submitting
  `processRoutingFee` (step c), the PathUSD is stranded at the sink. The
  bot does not retry on revert — production needs a retry queue.
- **Static rate.** `StaticRateConverter` is a configuration value; there is
  no oracle, no TWAP, no slippage protection. Production swaps the
  converter for an oracle/DEX reader and runs the PathUSD→GAZE swap before
  calling `processRoutingFee`.
- **Single-account.** The bot signs from one EOA. No nonce manager, no
  fee-bump on stuck transactions.

## Running tests

```
cd packages/settler
npx vitest run
```
