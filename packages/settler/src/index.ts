// Public surface.
//
// The settler bot has two concerns: subscribing to settle events and
// converting PathUSD → GAZE. Both are exported so consumers can wire
// alternative converters (oracle / DEX) without forking the bot itself.

export {
  StargazeSettler,
  type SettlerConfig,
  type SessionSettledLogArgs,
  STARGAZE_ESCROW_SESSION_SETTLED_ABI,
  BURN_CONTROLLER_PROCESS_FEE_ABI,
  ERC20_ALLOWANCE_ABI,
} from './settler.js';

export {
  StaticRateConverter,
  type PathUsdToGazeConverter,
} from './conversion.js';
