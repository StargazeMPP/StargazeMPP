/**
 * Off-chain PathUSD → GAZE conversion for the settler bot.
 *
 * v1 is a reference impl: the rate is static and injected at construction.
 * Production will replace `StaticRateConverter` with an oracle reader or
 * a DEX TWAP (and likely a swap action on the same call site).
 */
export interface PathUsdToGazeConverter {
  /** Convert a PathUSD amount (base units, 6 decimals) to a GAZE amount (base units, 18 decimals). */
  toGaze(pathUsdAmount: bigint): bigint;
}

/**
 * Static-rate converter. Multiplies `pathUsdAmount` by `rate` and returns
 * the result. `rate` is expressed in "GAZE base units per PathUSD base unit",
 * so the canonical 1 PathUSD = 1 GAZE pegging at the test rate is `1e12`
 * (i.e. `1e6 * 1e12 == 1e18`).
 */
export class StaticRateConverter implements PathUsdToGazeConverter {
  constructor(private readonly rate: bigint) {
    if (rate <= 0n) {
      throw new Error('StaticRateConverter: rate must be positive');
    }
  }

  toGaze(pathUsdAmount: bigint): bigint {
    return pathUsdAmount * this.rate;
  }
}
