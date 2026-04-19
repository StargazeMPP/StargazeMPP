use anchor_lang::prelude::*;

declare_id!("m6P7kwvXoET9n5B8DFGwwLEozXdv6jBJPdbMiW1TH1R");

/// StargazeAnchor — Solana-side mirror of the Tempo `StargazeRegistry`.
///
/// Responsibilities:
///   1. Register Solana-native providers (no $GAZE stake here; reputational
///      score is mirrored from Tempo via the CCIP bridge).
///   2. Persist x402 USDC receipts so the indexer can project them into
///      Postgres without re-fetching tx history.
///   3. Cast reputation votes that mirror to Tempo (CPI to the CCIP router
///      stubbed for now — wired in M4).
#[program]
pub mod stargaze_anchor {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, authority: Pubkey) -> Result<()> {
        let cfg = &mut ctx.accounts.config;
        cfg.authority = authority;
        cfg.provider_count = 0;
        cfg.bump = ctx.bumps.config;
        Ok(())
    }

    pub fn register_provider(
        ctx: Context<RegisterProvider>,
        provider_id: [u8; 32],
        category_hash: [u8; 32],
        meta_cid: [u8; 32],
    ) -> Result<()> {
        let provider = &mut ctx.accounts.provider;
        provider.owner = ctx.accounts.owner.key();
        provider.provider_id = provider_id;
        provider.category_hash = category_hash;
        provider.meta_cid = meta_cid;
        provider.reputation_score = 500; // neutral midpoint, mirrored from Tempo later
        provider.registered_at = Clock::get()?.unix_timestamp;
        provider.bump = ctx.bumps.provider;

        let cfg = &mut ctx.accounts.config;
        cfg.provider_count = cfg.provider_count.saturating_add(1);

        emit!(ProviderRegistered {
            provider_id,
            owner: provider.owner,
            category_hash,
            meta_cid,
        });
        Ok(())
    }

    pub fn cast_reputation_vote(
        ctx: Context<CastReputationVote>,
        provider_id: [u8; 32],
        accurate: bool,
    ) -> Result<()> {
        emit!(ReputationVoted {
            provider_id,
            voter: ctx.accounts.voter.key(),
            accurate,
        });
        // TODO: CPI to CCIP router so the burn registers on Tempo (M4).
        Ok(())
    }

    pub fn record_x402_receipt(
        ctx: Context<RecordX402Receipt>,
        session_id: [u8; 32],
        provider_id: [u8; 32],
        amount: u64,
    ) -> Result<()> {
        let receipt = &mut ctx.accounts.receipt;
        receipt.session_id = session_id;
        receipt.provider_id = provider_id;
        receipt.payer = ctx.accounts.payer.key();
        receipt.amount = amount;
        receipt.paid_at = Clock::get()?.unix_timestamp;
        receipt.bump = ctx.bumps.receipt;

        emit!(X402ReceiptRecorded {
            session_id,
            provider_id,
            payer: receipt.payer,
            amount,
            paid_at: receipt.paid_at,
        });
        Ok(())
    }

    pub fn ccip_mirror_score(
        ctx: Context<CcipMirrorScore>,
        provider_id: [u8; 32],
        new_score: u16,
    ) -> Result<()> {
        require!(new_score <= 1000, StargazeAnchorError::ScoreOutOfRange);
        require_keys_eq!(
            ctx.accounts.ccip_router.key(),
            ctx.accounts.config.authority,
            StargazeAnchorError::Unauthorized
        );
        let provider = &mut ctx.accounts.provider;
        provider.reputation_score = new_score;
        emit!(ReputationMirrored {
            provider_id,
            score: new_score,
        });
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        init,
        payer = payer,
        space = 8 + Config::INIT_SPACE,
        seeds = [b"config"],
        bump
    )]
    pub config: Account<'info, Config>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(provider_id: [u8; 32])]
pub struct RegisterProvider<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(
        mut,
        seeds = [b"config"],
        bump = config.bump
    )]
    pub config: Account<'info, Config>,
    #[account(
        init,
        payer = owner,
        space = 8 + Provider::INIT_SPACE,
        seeds = [b"provider", provider_id.as_ref()],
        bump
    )]
    pub provider: Account<'info, Provider>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(provider_id: [u8; 32])]
pub struct CastReputationVote<'info> {
    pub voter: Signer<'info>,
    #[account(
        seeds = [b"provider", provider_id.as_ref()],
        bump = provider.bump
    )]
    pub provider: Account<'info, Provider>,
}

#[derive(Accounts)]
#[instruction(session_id: [u8; 32], provider_id: [u8; 32])]
pub struct RecordX402Receipt<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        init,
        payer = payer,
        space = 8 + X402Receipt::INIT_SPACE,
        seeds = [b"x402", session_id.as_ref(), provider_id.as_ref()],
        bump
    )]
    pub receipt: Account<'info, X402Receipt>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(provider_id: [u8; 32])]
pub struct CcipMirrorScore<'info> {
    pub ccip_router: Signer<'info>,
    #[account(seeds = [b"config"], bump = config.bump)]
    pub config: Account<'info, Config>,
    #[account(
        mut,
        seeds = [b"provider", provider_id.as_ref()],
        bump = provider.bump
    )]
    pub provider: Account<'info, Provider>,
}

#[account]
#[derive(InitSpace)]
pub struct Config {
    pub authority: Pubkey,
    pub provider_count: u64,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct Provider {
    pub owner: Pubkey,
    pub provider_id: [u8; 32],
    pub category_hash: [u8; 32],
    pub meta_cid: [u8; 32],
    pub reputation_score: u16,
    pub registered_at: i64,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct X402Receipt {
    pub session_id: [u8; 32],
    pub provider_id: [u8; 32],
    pub payer: Pubkey,
    pub amount: u64,
    pub paid_at: i64,
    pub bump: u8,
}

#[event]
pub struct ProviderRegistered {
    pub provider_id: [u8; 32],
    pub owner: Pubkey,
    pub category_hash: [u8; 32],
    pub meta_cid: [u8; 32],
}

#[event]
pub struct ReputationVoted {
    pub provider_id: [u8; 32],
    pub voter: Pubkey,
    pub accurate: bool,
}

#[event]
pub struct X402ReceiptRecorded {
    pub session_id: [u8; 32],
    pub provider_id: [u8; 32],
    pub payer: Pubkey,
    pub amount: u64,
    pub paid_at: i64,
}

#[event]
pub struct ReputationMirrored {
    pub provider_id: [u8; 32],
    pub score: u16,
}

#[error_code]
pub enum StargazeAnchorError {
    #[msg("Reputation score must be between 0 and 1000.")]
    ScoreOutOfRange,
    #[msg("Caller is not authorised for this instruction.")]
    Unauthorized,
}
