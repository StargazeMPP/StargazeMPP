use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

declare_id!("m6P7kwvXoET9n5B8DFGwwLEozXdv6jBJPdbMiW1TH1R");

/// Minimum stake required for a Solana-native provider to be considered active
/// (50 GAZE at 6 decimals).
#[constant]
pub const MIN_STAKE_DEFAULT: u64 = 50_000_000;
/// Stake threshold for a provider to be considered "verified"
/// (500 GAZE at 6 decimals).
#[constant]
pub const VERIFIED_STAKE_DEFAULT: u64 = 500_000_000;
/// Default unstake cooldown window (7 days).
#[constant]
pub const COOLDOWN_DEFAULT_SECS: i64 = 7 * 86_400;

/// Fixed burn destination: the canonical Solana incinerator address. Tokens
/// sent here are unrecoverable.
pub const BURN_DESTINATION: Pubkey =
    anchor_lang::solana_program::pubkey!("1nc1nerator11111111111111111111111111111111");

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

    /// Dispatch a reputation snapshot for `provider_id` to the Tempo receiver
    /// via Chainlink CCIP. The on-chain `Provider.reputation_score` is the
    /// authoritative value being mirrored.
    ///
    /// Payload schema (matches `StargazeCcipReceiver.ccipReceive`):
    ///   `abi.encode(bytes32 providerId, uint16 score)` — 64 bytes total.
    ///
    /// CPI into the Chainlink router is wired in M4; for now the message is
    /// emitted as a `CcipDispatched` event so the indexer can observe it.
    /// The router program id is supplied via the `CHAINLINK_CCIP_PROGRAM_ID`
    /// env var at deploy time.
    pub fn dispatch_reputation_to_tempo(
        ctx: Context<DispatchReputationToTempo>,
        provider_id: [u8; 32],
        dest_chain_selector: u64,
        receiver: Vec<u8>,
        extra_args: Vec<u8>,
    ) -> Result<()> {
        require_keys_eq!(
            ctx.accounts.sender.key(),
            ctx.accounts.config.authority,
            StargazeAnchorError::Unauthorized
        );

        let score = ctx.accounts.provider.reputation_score;

        let mut payload = Vec::with_capacity(64);
        payload.extend_from_slice(&provider_id);
        payload.extend_from_slice(&[0u8; 30]);
        payload.extend_from_slice(&score.to_be_bytes());

        emit!(CcipDispatched {
            provider_id,
            score,
            dest_chain_selector,
            receiver,
            payload,
            extra_args,
        });

        // M4: CPI to ctx.accounts.ccip_router_program goes here.
        let _ = &ctx.accounts.ccip_router_program;

        Ok(())
    }

    /// Initialise the `StakingConfig` singleton. Admin only (must match
    /// the existing `Config.authority`). If `stake_mint == Pubkey::default()`
    /// the pool token account is not created — call `set_stake_mint` later
    /// once the pump.fun launch has produced the real mint.
    pub fn init_staking(
        ctx: Context<InitStaking>,
        stake_mint: Pubkey,
        min_stake: u64,
        verified_stake: u64,
        cooldown_secs: i64,
    ) -> Result<()> {
        require_keys_eq!(
            ctx.accounts.authority.key(),
            ctx.accounts.config.authority,
            StargazeAnchorError::Unauthorized
        );
        let staking = &mut ctx.accounts.staking_config;
        staking.authority = ctx.accounts.config.authority;
        staking.stake_mint = stake_mint;
        staking.min_stake = min_stake;
        staking.verified_stake = verified_stake;
        staking.cooldown_secs = cooldown_secs;
        staking.bump = ctx.bumps.staking_config;

        emit!(StakingInitialized {
            stake_mint,
            min_stake,
            verified_stake,
            cooldown_secs,
        });
        Ok(())
    }

    /// One-shot setter for the stake mint, used when staking is initialised
    /// before the pump.fun launch has produced the SPL mint. Also creates the
    /// pool token account on first call. Subsequent calls are rejected unless
    /// the mint is still `Pubkey::default()` on entry.
    pub fn set_stake_mint(ctx: Context<SetStakeMint>, new_mint: Pubkey) -> Result<()> {
        require_keys_eq!(
            ctx.accounts.authority.key(),
            ctx.accounts.staking_config.authority,
            StargazeAnchorError::Unauthorized
        );
        require!(
            ctx.accounts.staking_config.stake_mint == Pubkey::default(),
            StargazeAnchorError::StakeMintAlreadySet
        );
        require!(
            new_mint != Pubkey::default(),
            StargazeAnchorError::StakeMintUnset
        );
        require_keys_eq!(
            ctx.accounts.stake_mint.key(),
            new_mint,
            StargazeAnchorError::StakeMintUnset
        );
        require_keys_eq!(
            ctx.accounts.pool_token_account.mint,
            new_mint,
            StargazeAnchorError::StakeMintUnset
        );
        require_keys_eq!(
            ctx.accounts.pool_token_account.owner,
            ctx.accounts.stake_pool_authority.key(),
            StargazeAnchorError::Unauthorized
        );

        ctx.accounts.staking_config.stake_mint = new_mint;

        emit!(StakeMintSet { stake_mint: new_mint });
        Ok(())
    }

    /// Stake `amount` base units of $GAZE against `provider_id`. The caller
    /// is the staker; tokens move from `staker_ata` into the pool. The
    /// per-staker `StakeAccount` is created on first stake.
    pub fn stake(
        ctx: Context<Stake>,
        provider_id: [u8; 32],
        amount: u64,
    ) -> Result<()> {
        require!(amount > 0, StargazeAnchorError::StakeAmountZero);
        require!(
            ctx.accounts.staking_config.stake_mint != Pubkey::default(),
            StargazeAnchorError::StakeMintUnset
        );

        let stake_acct = &mut ctx.accounts.stake_account;
        if stake_acct.owner == Pubkey::default() {
            stake_acct.provider_id = provider_id;
            stake_acct.owner = ctx.accounts.staker.key();
            stake_acct.bump = ctx.bumps.stake_account;
        }

        let cpi_accounts = Transfer {
            from: ctx.accounts.staker_ata.to_account_info(),
            to: ctx.accounts.pool_token_account.to_account_info(),
            authority: ctx.accounts.staker.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts);
        token::transfer(cpi_ctx, amount)?;

        stake_acct.amount = stake_acct
            .amount
            .checked_add(amount)
            .ok_or(StargazeAnchorError::InsufficientStake)?;

        emit!(Staked {
            provider_id,
            owner: stake_acct.owner,
            amount,
            total: stake_acct.amount,
        });
        Ok(())
    }

    /// Queue `amount` for unstake. The amount remains in the pool until
    /// the cooldown expires and `claim_unstake` is called. Multiple
    /// consecutive requests accumulate into `cooldown_amount` and reset the
    /// cooldown clock to the latest call.
    pub fn request_unstake(
        ctx: Context<MutateStake>,
        provider_id: [u8; 32],
        amount: u64,
    ) -> Result<()> {
        require!(amount > 0, StargazeAnchorError::StakeAmountZero);
        let stake_acct = &mut ctx.accounts.stake_account;
        let available = stake_acct
            .amount
            .checked_sub(stake_acct.cooldown_amount)
            .ok_or(StargazeAnchorError::InsufficientStake)?;
        require!(amount <= available, StargazeAnchorError::InsufficientStake);

        let now = Clock::get()?.unix_timestamp;
        stake_acct.cooldown_amount = stake_acct
            .cooldown_amount
            .checked_add(amount)
            .ok_or(StargazeAnchorError::InsufficientStake)?;
        // Reset the cooldown clock to the latest request — simplest model.
        stake_acct.cooldown_start_ts = now;

        let cooldown_until = now
            .checked_add(ctx.accounts.staking_config.cooldown_secs)
            .unwrap_or(i64::MAX);

        emit!(UnstakeRequested {
            provider_id,
            owner: stake_acct.owner,
            amount,
            cooldown_until,
        });
        let _ = provider_id;
        Ok(())
    }

    /// Claim queued unstake amount once the cooldown has elapsed. Transfers
    /// `cooldown_amount` from the pool back to the staker and clears the
    /// cooldown state.
    pub fn claim_unstake(
        ctx: Context<ClaimUnstake>,
        provider_id: [u8; 32],
    ) -> Result<()> {
        let cooldown_amount = ctx.accounts.stake_account.cooldown_amount;
        require!(
            cooldown_amount > 0,
            StargazeAnchorError::NoCooldownInProgress
        );
        let now = Clock::get()?.unix_timestamp;
        let elapsed = now.saturating_sub(ctx.accounts.stake_account.cooldown_start_ts);
        require!(
            elapsed >= ctx.accounts.staking_config.cooldown_secs,
            StargazeAnchorError::CooldownActive
        );

        let bump = ctx.bumps.stake_pool_authority;
        let seeds: &[&[u8]] = &[b"stake_pool_authority", &[bump]];
        let signer_seeds = &[seeds];
        let cpi_accounts = Transfer {
            from: ctx.accounts.pool_token_account.to_account_info(),
            to: ctx.accounts.staker_ata.to_account_info(),
            authority: ctx.accounts.stake_pool_authority.to_account_info(),
        };
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
            signer_seeds,
        );
        token::transfer(cpi_ctx, cooldown_amount)?;

        let stake_acct = &mut ctx.accounts.stake_account;
        stake_acct.amount = stake_acct
            .amount
            .checked_sub(cooldown_amount)
            .ok_or(StargazeAnchorError::InsufficientStake)?;
        stake_acct.cooldown_amount = 0;
        stake_acct.cooldown_start_ts = 0;

        emit!(Unstaked {
            provider_id,
            owner: stake_acct.owner,
            amount: cooldown_amount,
        });
        Ok(())
    }

    /// Admin-only slash. Transfers up to `amount` base units from the pool
    /// to the canonical Solana incinerator burn address. The amount is
    /// capped at the target's currently-available (non-cooldown) stake to
    /// keep accounting consistent.
    pub fn slash(
        ctx: Context<Slash>,
        provider_id: [u8; 32],
        staker: Pubkey,
        amount: u64,
    ) -> Result<()> {
        require_keys_eq!(
            ctx.accounts.authority.key(),
            ctx.accounts.staking_config.authority,
            StargazeAnchorError::Unauthorized
        );
        require!(amount > 0, StargazeAnchorError::StakeAmountZero);
        require_keys_eq!(
            ctx.accounts.stake_account.owner,
            staker,
            StargazeAnchorError::Unauthorized
        );
        require_keys_eq!(
            ctx.accounts.burn_destination_ata.owner,
            BURN_DESTINATION,
            StargazeAnchorError::Unauthorized
        );

        let available = ctx
            .accounts
            .stake_account
            .amount
            .saturating_sub(ctx.accounts.stake_account.cooldown_amount);
        let to_slash = amount.min(available);
        require!(to_slash > 0, StargazeAnchorError::InsufficientStake);

        let bump = ctx.bumps.stake_pool_authority;
        let seeds: &[&[u8]] = &[b"stake_pool_authority", &[bump]];
        let signer_seeds = &[seeds];
        let cpi_accounts = Transfer {
            from: ctx.accounts.pool_token_account.to_account_info(),
            to: ctx.accounts.burn_destination_ata.to_account_info(),
            authority: ctx.accounts.stake_pool_authority.to_account_info(),
        };
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
            signer_seeds,
        );
        token::transfer(cpi_ctx, to_slash)?;

        let stake_acct = &mut ctx.accounts.stake_account;
        stake_acct.amount = stake_acct
            .amount
            .checked_sub(to_slash)
            .ok_or(StargazeAnchorError::InsufficientStake)?;

        emit!(Slashed {
            provider_id,
            owner: staker,
            amount: to_slash,
            destination: BURN_DESTINATION,
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

#[derive(Accounts)]
#[instruction(provider_id: [u8; 32])]
pub struct DispatchReputationToTempo<'info> {
    pub sender: Signer<'info>,
    #[account(seeds = [b"config"], bump = config.bump)]
    pub config: Account<'info, Config>,
    #[account(
        seeds = [b"provider", provider_id.as_ref()],
        bump = provider.bump
    )]
    pub provider: Account<'info, Provider>,
    /// CHECK: Chainlink CCIP Solana router program. Not yet invoked — passed
    /// in by the client so a future CPI can target the configured router.
    pub ccip_router_program: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct InitStaking<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(seeds = [b"config"], bump = config.bump)]
    pub config: Account<'info, Config>,
    #[account(
        init,
        payer = authority,
        space = 8 + StakingConfig::INIT_SPACE,
        seeds = [b"staking_config"],
        bump
    )]
    pub staking_config: Account<'info, StakingConfig>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct SetStakeMint<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        mut,
        seeds = [b"staking_config"],
        bump = staking_config.bump
    )]
    pub staking_config: Account<'info, StakingConfig>,
    pub stake_mint: Account<'info, Mint>,
    /// CHECK: PDA signer for the pool token account. Address is verified
    /// via the seeds constraint.
    #[account(seeds = [b"stake_pool_authority"], bump)]
    pub stake_pool_authority: UncheckedAccount<'info>,
    #[account(
        init_if_needed,
        payer = authority,
        associated_token::mint = stake_mint,
        associated_token::authority = stake_pool_authority,
    )]
    pub pool_token_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(provider_id: [u8; 32])]
pub struct Stake<'info> {
    #[account(mut)]
    pub staker: Signer<'info>,
    #[account(seeds = [b"staking_config"], bump = staking_config.bump)]
    pub staking_config: Account<'info, StakingConfig>,
    #[account(
        init_if_needed,
        payer = staker,
        space = 8 + StakeAccount::INIT_SPACE,
        seeds = [b"stake", provider_id.as_ref(), staker.key().as_ref()],
        bump
    )]
    pub stake_account: Account<'info, StakeAccount>,
    #[account(
        mut,
        constraint = stake_mint.key() == staking_config.stake_mint @ StargazeAnchorError::StakeMintUnset
    )]
    pub stake_mint: Account<'info, Mint>,
    #[account(
        mut,
        constraint = staker_ata.owner == staker.key() @ StargazeAnchorError::Unauthorized,
        constraint = staker_ata.mint == stake_mint.key() @ StargazeAnchorError::StakeMintUnset
    )]
    pub staker_ata: Account<'info, TokenAccount>,
    /// CHECK: PDA signer for the pool token account. Address is verified
    /// via the seeds constraint.
    #[account(seeds = [b"stake_pool_authority"], bump)]
    pub stake_pool_authority: UncheckedAccount<'info>,
    #[account(
        mut,
        constraint = pool_token_account.owner == stake_pool_authority.key() @ StargazeAnchorError::Unauthorized,
        constraint = pool_token_account.mint == stake_mint.key() @ StargazeAnchorError::StakeMintUnset
    )]
    pub pool_token_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(provider_id: [u8; 32])]
pub struct MutateStake<'info> {
    pub staker: Signer<'info>,
    #[account(seeds = [b"staking_config"], bump = staking_config.bump)]
    pub staking_config: Account<'info, StakingConfig>,
    #[account(
        mut,
        seeds = [b"stake", provider_id.as_ref(), staker.key().as_ref()],
        bump = stake_account.bump,
        constraint = stake_account.owner == staker.key() @ StargazeAnchorError::Unauthorized
    )]
    pub stake_account: Account<'info, StakeAccount>,
}

#[derive(Accounts)]
#[instruction(provider_id: [u8; 32])]
pub struct ClaimUnstake<'info> {
    #[account(mut)]
    pub staker: Signer<'info>,
    #[account(seeds = [b"staking_config"], bump = staking_config.bump)]
    pub staking_config: Account<'info, StakingConfig>,
    #[account(
        mut,
        seeds = [b"stake", provider_id.as_ref(), staker.key().as_ref()],
        bump = stake_account.bump,
        constraint = stake_account.owner == staker.key() @ StargazeAnchorError::Unauthorized
    )]
    pub stake_account: Account<'info, StakeAccount>,
    #[account(
        constraint = stake_mint.key() == staking_config.stake_mint @ StargazeAnchorError::StakeMintUnset
    )]
    pub stake_mint: Account<'info, Mint>,
    #[account(
        mut,
        constraint = staker_ata.owner == staker.key() @ StargazeAnchorError::Unauthorized,
        constraint = staker_ata.mint == stake_mint.key() @ StargazeAnchorError::StakeMintUnset
    )]
    pub staker_ata: Account<'info, TokenAccount>,
    /// CHECK: PDA signer for the pool token account. Address is verified
    /// via the seeds constraint.
    #[account(seeds = [b"stake_pool_authority"], bump)]
    pub stake_pool_authority: UncheckedAccount<'info>,
    #[account(
        mut,
        constraint = pool_token_account.owner == stake_pool_authority.key() @ StargazeAnchorError::Unauthorized,
        constraint = pool_token_account.mint == stake_mint.key() @ StargazeAnchorError::StakeMintUnset
    )]
    pub pool_token_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
#[instruction(provider_id: [u8; 32], staker: Pubkey)]
pub struct Slash<'info> {
    pub authority: Signer<'info>,
    #[account(seeds = [b"staking_config"], bump = staking_config.bump)]
    pub staking_config: Account<'info, StakingConfig>,
    #[account(
        mut,
        seeds = [b"stake", provider_id.as_ref(), staker.as_ref()],
        bump = stake_account.bump
    )]
    pub stake_account: Account<'info, StakeAccount>,
    #[account(
        constraint = stake_mint.key() == staking_config.stake_mint @ StargazeAnchorError::StakeMintUnset
    )]
    pub stake_mint: Account<'info, Mint>,
    /// CHECK: PDA signer for the pool token account.
    #[account(seeds = [b"stake_pool_authority"], bump)]
    pub stake_pool_authority: UncheckedAccount<'info>,
    #[account(
        mut,
        constraint = pool_token_account.owner == stake_pool_authority.key() @ StargazeAnchorError::Unauthorized,
        constraint = pool_token_account.mint == stake_mint.key() @ StargazeAnchorError::StakeMintUnset
    )]
    pub pool_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = burn_destination_ata.mint == stake_mint.key() @ StargazeAnchorError::StakeMintUnset
    )]
    pub burn_destination_ata: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
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
pub struct StakingConfig {
    pub authority: Pubkey,
    pub stake_mint: Pubkey,
    pub min_stake: u64,
    pub verified_stake: u64,
    pub cooldown_secs: i64,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct StakeAccount {
    pub provider_id: [u8; 32],
    pub owner: Pubkey,
    pub amount: u64,
    pub cooldown_amount: u64,
    pub cooldown_start_ts: i64,
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

#[event]
pub struct CcipDispatched {
    pub provider_id: [u8; 32],
    pub score: u16,
    pub dest_chain_selector: u64,
    pub receiver: Vec<u8>,
    pub payload: Vec<u8>,
    pub extra_args: Vec<u8>,
}

#[event]
pub struct Staked {
    pub provider_id: [u8; 32],
    pub owner: Pubkey,
    pub amount: u64,
    pub total: u64,
}

#[event]
pub struct UnstakeRequested {
    pub provider_id: [u8; 32],
    pub owner: Pubkey,
    pub amount: u64,
    pub cooldown_until: i64,
}

#[event]
pub struct Unstaked {
    pub provider_id: [u8; 32],
    pub owner: Pubkey,
    pub amount: u64,
}

#[event]
pub struct Slashed {
    pub provider_id: [u8; 32],
    pub owner: Pubkey,
    pub amount: u64,
    pub destination: Pubkey,
}

#[event]
pub struct StakingInitialized {
    pub stake_mint: Pubkey,
    pub min_stake: u64,
    pub verified_stake: u64,
    pub cooldown_secs: i64,
}

#[event]
pub struct StakeMintSet {
    pub stake_mint: Pubkey,
}

#[error_code]
pub enum StargazeAnchorError {
    #[msg("Reputation score must be between 0 and 1000.")]
    ScoreOutOfRange,
    #[msg("Caller is not authorised for this instruction.")]
    Unauthorized,
    #[msg("Stake amount must be greater than zero.")]
    StakeAmountZero,
    #[msg("Stake mint has not been configured yet.")]
    StakeMintUnset,
    #[msg("Stake mint is already configured; one-shot setter rejected.")]
    StakeMintAlreadySet,
    #[msg("Insufficient stake to satisfy the requested amount.")]
    InsufficientStake,
    #[msg("Cooldown period has not elapsed yet.")]
    CooldownActive,
    #[msg("No cooldown is currently in progress for this stake account.")]
    NoCooldownInProgress,
}
