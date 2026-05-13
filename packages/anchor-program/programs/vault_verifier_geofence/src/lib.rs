use anchor_lang::prelude::*;
use vault_verifier_core::{verify_groth16, VerifierError};

declare_id!("9d5rDusSqBH44dnJ4CQnR885xUoDq9NevgDHcsjYBaoD");

// === DEV VKEY — DO NOT DEPLOY TO MAINNET ===
// Generated from packages/vault-circuits/build/geofence_vkey.json
// via packages/vault-circuits/scripts/emit-rust-vkey.mjs.
// Regenerate after every trusted-setup ceremony.
mod vkey;
use vkey::VKEY;

#[program]
pub mod vault_verifier_geofence {
    use super::*;

    /// Verify a Groth16 proof for the geofence circuit (N=32 bit-width, 4
    /// public signals — minLat, maxLat, minLon, maxLon).
    /// On success: ix succeeds with no state change. Verifier programs are pure
    /// dispatchers; the calling program records the proof receipt.
    pub fn verify(
        _ctx: Context<NoAccounts>,
        proof_bytes: [u8; 256],
        public_signals: Vec<[u8; 32]>,
    ) -> Result<()> {
        let signals: [[u8; 32]; 4] = public_signals
            .as_slice()
            .try_into()
            .map_err(|_| ErrorCode::WrongSignalCount)?;
        verify_groth16::<4>(&VKEY, &proof_bytes, &signals).map_err(|e| match e {
            VerifierError::SignalCountMismatch => ErrorCode::WrongSignalCount.into(),
            VerifierError::ProofInvalid => ErrorCode::ProofInvalid.into(),
        })
    }
}

#[derive(Accounts)]
pub struct NoAccounts {}

#[error_code]
pub enum ErrorCode {
    #[msg("public signals length does not match circuit shape")]
    WrongSignalCount,
    #[msg("groth16 proof did not verify")]
    ProofInvalid,
}
