import StargazeEscrow from './abi/StargazeEscrow.json' with { type: 'json' };
import StargazeRegistry from './abi/StargazeRegistry.json' with { type: 'json' };
import PrivacyVaultRegistry from './abi/PrivacyVaultRegistry.json' with { type: 'json' };
import StargazeCcipReceiver from './abi/StargazeCcipReceiver.json' with { type: 'json' };

export const STARGAZE_ESCROW_ABI = StargazeEscrow.abi;
export const STARGAZE_REGISTRY_ABI = StargazeRegistry.abi;
export const PRIVACY_VAULT_REGISTRY_ABI = PrivacyVaultRegistry.abi;
export const STARGAZE_CCIP_RECEIVER_ABI = StargazeCcipReceiver.abi;

export const ABI_BYTECODE_HASHES = {
  StargazeEscrow: StargazeEscrow.bytecodeHash,
  StargazeRegistry: StargazeRegistry.bytecodeHash,
  PrivacyVaultRegistry: PrivacyVaultRegistry.bytecodeHash,
  StargazeCcipReceiver: StargazeCcipReceiver.bytecodeHash,
} as const;
