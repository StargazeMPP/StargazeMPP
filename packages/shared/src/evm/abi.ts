import GAZEToken from './abi/GAZEToken.json' with { type: 'json' };
import BurnController from './abi/BurnController.json' with { type: 'json' };
import StargazeEscrow from './abi/StargazeEscrow.json' with { type: 'json' };
import StargazeRegistry from './abi/StargazeRegistry.json' with { type: 'json' };
import PrivacyVaultRegistry from './abi/PrivacyVaultRegistry.json' with { type: 'json' };

export const GAZE_TOKEN_ABI = GAZEToken.abi;
export const BURN_CONTROLLER_ABI = BurnController.abi;
export const STARGAZE_ESCROW_ABI = StargazeEscrow.abi;
export const STARGAZE_REGISTRY_ABI = StargazeRegistry.abi;
export const PRIVACY_VAULT_REGISTRY_ABI = PrivacyVaultRegistry.abi;

export const ABI_BYTECODE_HASHES = {
  GAZEToken: GAZEToken.bytecodeHash,
  BurnController: BurnController.bytecodeHash,
  StargazeEscrow: StargazeEscrow.bytecodeHash,
  StargazeRegistry: StargazeRegistry.bytecodeHash,
  PrivacyVaultRegistry: PrivacyVaultRegistry.bytecodeHash,
} as const;
