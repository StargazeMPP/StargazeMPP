use anchor_lang::prelude::*;

declare_id!("H2T3Amf7eTpeQQbzHkhTv5buCkfdc8bS41YwfgVkVsGn");

#[program]
pub mod vault_verifier_buyer_key {
    use super::*;

    pub fn verify(
        _ctx: Context<NoAccounts>,
        proof_bytes: [u8; 256],
        public_signals: Vec<[u8; 32]>,
    ) -> Result<()> {
        let _ = (proof_bytes, public_signals);
        Err(ErrorCode::CircuitNotFinalised.into())
    }
}

#[derive(Accounts)]
pub struct NoAccounts {}

#[error_code]
pub enum ErrorCode {
    #[msg("buyer-key circuit is not finalised; verifier ships as an always-rejecting stub")]
    CircuitNotFinalised,
}
