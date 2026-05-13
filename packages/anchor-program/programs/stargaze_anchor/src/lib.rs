use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{self, Burn, Mint, Token, TokenAccount, Transfer};

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
/// Fixed reputation-vote burn amount: 1 GAZE at 6 decimals. One vote = one
/// token, irrevocably burned from the voter's ATA. Surfaced through the IDL so
/// the indexer and the client SDK can both reference the same constant.
#[constant]
pub const VOTE_BURN_AMOUNT: u64 = 1_000_000;

/// Fixed burn destination: the canonical Solana incinerator address. Tokens
/// sent here are unrecoverable.
pub const BURN_DESTINATION: Pubkey =
    anchor_lang::solana_program::pubkey!("1nc1nerator11111111111111111111111111111111");

// ============ ESCROW: constants ============
/// Domain separator for the off-chain voucher signing scheme. Twenty-one
/// ASCII bytes; surfaced through the IDL so the off-chain SDK can copy the
/// same literal byte-for-byte. The voucher message starts with this prefix
/// followed by `session_id || agent_wallet || provider_id ||
/// cumulative_amount_le || nonce_le` for a total of 133 bytes.
#[constant]
pub const VOUCHER_DOMAIN_TAG: [u8; 21] = *b"StargazeMPP/Voucher/1";

/// Total length of the voucher message bytes that the off-chain agent signs
/// and the on-chain program verifies (see `VOUCHER_DOMAIN_TAG`).
#[constant]
pub const VOUCHER_MESSAGE_LEN: u32 = 133;

/// Routing-fee basis points. 2% of every voucher amount is diverted from the
/// provider payout into the singleton routing-fee USDC vault.
#[constant]
pub const ROUTING_FEE_BPS: u16 = 200;

/// Conversion of routing-fee USDC into $GAZE for the burn ladder is handled
/// in a separate (admin-only) instruction `convert_routing_fees_to_gaze`
/// which is **NOT** in scope here. The planned flow is:
///   1. Drain `routing_fee_vault_ata` (USDC).
///   2. Swap USDC -> $GAZE via Jupiter (or comparable router).
///   3. CPI into `process_routing_fee_burn` with the resulting $GAZE.
/// Until that instruction lands, settled routing fees accumulate as USDC
/// in the vault.

/// Anchor `global:verify` ix discriminator — `sha256("global:verify")[..8]`.
/// All three vault-verifier programs (`vault_verifier_aggregate_sum`,
/// `vault_verifier_geofence`, `vault_verifier_buyer_key`) expose the same
/// `verify(proof_bytes, public_signals)` entrypoint, so a single constant
/// covers the manual-CPI path for every per-circuit verifier.
const VERIFY_IX_DISCRIMINATOR: [u8; 8] =
    [0x85, 0xa1, 0x8d, 0x30, 0x78, 0xc6, 0x58, 0x96];

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

    /// Dispatch a per-staker stake snapshot for `(provider_id, owner)` to the
    /// Tempo `StargazeStakeMirror` receiver via Chainlink CCIP. The on-chain
    /// `StakeAccount.amount` is the authoritative value being mirrored.
    ///
    /// Payload schema (matches `StargazeStakeMirror.ccipReceive`):
    ///   `abi.encode(bytes32 providerId, address owner, uint256 amount)` —
    ///   96 bytes total.
    ///
    /// Note: `owner` is the Solana staker pubkey truncated to the bottom 20
    /// bytes to fit Solidity's 32-byte address slot; the Tempo mirror treats
    /// it as a per-Solana-staker key, NOT an EVM address. The Tempo side
    /// never needs to send a transaction from this address.
    ///
    /// CPI into the Chainlink router is wired in M4; for now the message is
    /// emitted as a `StakeDispatched` event so the indexer can observe it.
    pub fn dispatch_stake_to_tempo(
        ctx: Context<DispatchStakeToTempo>,
        provider_id: [u8; 32],
        owner: Pubkey,
        dest_chain_selector: u64,
        receiver: Vec<u8>,
        extra_args: Vec<u8>,
    ) -> Result<()> {
        require_keys_eq!(
            ctx.accounts.sender.key(),
            ctx.accounts.staking_config.authority,
            StargazeAnchorError::Unauthorized
        );

        let amount = ctx.accounts.stake_account.amount;

        // ABI: bytes32 providerId || address owner (right-aligned in 32 bytes)
        //   || uint256 amount (big-endian). 96 bytes total.
        let mut payload = Vec::with_capacity(96);
        payload.extend_from_slice(&provider_id);
        // Solidity address is 20 bytes right-aligned in a 32-byte slot: 12
        // leading zero bytes, then the bottom 20 bytes of the Solana pubkey.
        payload.extend_from_slice(&[0u8; 12]);
        let owner_bytes = owner.to_bytes();
        payload.extend_from_slice(&owner_bytes[12..32]);
        // uint256 amount as big-endian, left-padded with 24 zero bytes.
        payload.extend_from_slice(&[0u8; 24]);
        payload.extend_from_slice(&amount.to_be_bytes());

        emit!(StakeDispatched {
            provider_id,
            owner,
            amount,
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
        staking.total_routing_fee_burned = 0;
        staking.total_routing_fee_to_stakers = 0;
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

    /// Process a routing-fee tranche through the burn ladder. Splits `amount`
    /// 50/50: half is permanently burned via `token::burn` (reducing SPL
    /// supply), half is transferred to the staker-reward pool ATA controlled
    /// by the `staker_reward_pool_authority` PDA. On odd amounts the extra
    /// base unit is routed to stakers (`to_stakers = amount - amount/2`).
    ///
    /// The reward pool only accumulates here — distribution is deferred.
    /// Defer: staker reward distribution mechanism (pull-based Merkle vs
    /// push-based proportional) is unresolved.
    ///
    /// M4: the `authority` admin gate is swapped for the CCIP fan-out so the
    /// Tempo routing fee can be processed without a privileged signer.
    pub fn process_routing_fee_burn(
        ctx: Context<ProcessRoutingFeeBurn>,
        amount: u64,
    ) -> Result<()> {
        require_keys_eq!(
            ctx.accounts.authority.key(),
            ctx.accounts.staking_config.authority,
            StargazeAnchorError::Unauthorized
        );
        require!(amount > 0, StargazeAnchorError::StakeAmountZero);
        require!(
            ctx.accounts.staking_config.stake_mint != Pubkey::default(),
            StargazeAnchorError::StakeMintUnset
        );

        let burned = amount / 2;
        // Odd amount: the extra base unit goes to stakers, not the burn.
        let to_stakers = amount
            .checked_sub(burned)
            .ok_or(StargazeAnchorError::InsufficientStake)?;

        if burned > 0 {
            let cpi_accounts = Burn {
                mint: ctx.accounts.stake_mint.to_account_info(),
                from: ctx.accounts.authority_ata.to_account_info(),
                authority: ctx.accounts.authority.to_account_info(),
            };
            let cpi_ctx =
                CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts);
            token::burn(cpi_ctx, burned)?;
        }

        if to_stakers > 0 {
            let cpi_accounts = Transfer {
                from: ctx.accounts.authority_ata.to_account_info(),
                to: ctx.accounts.staker_reward_pool_ata.to_account_info(),
                authority: ctx.accounts.authority.to_account_info(),
            };
            let cpi_ctx =
                CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts);
            token::transfer(cpi_ctx, to_stakers)?;
        }

        let staking = &mut ctx.accounts.staking_config;
        staking.total_routing_fee_burned = staking
            .total_routing_fee_burned
            .checked_add(burned)
            .ok_or(StargazeAnchorError::InsufficientStake)?;
        staking.total_routing_fee_to_stakers = staking
            .total_routing_fee_to_stakers
            .checked_add(to_stakers)
            .ok_or(StargazeAnchorError::InsufficientStake)?;

        emit!(RoutingFeeProcessed { burned, to_stakers });
        Ok(())
    }

    /// Burn one $GAZE token from the voter's ATA as the on-chain cost of
    /// casting a reputation vote against `provider_id`. The vote itself is
    /// emitted upstream via `cast_reputation_vote` / Tempo; this instruction
    /// only enforces the token-burn cost. Insufficient balance bubbles up as
    /// a token-program error.
    pub fn reputation_vote_burn(
        ctx: Context<ReputationVoteBurn>,
        provider_id: [u8; 32],
    ) -> Result<()> {
        require!(
            ctx.accounts.staking_config.stake_mint != Pubkey::default(),
            StargazeAnchorError::StakeMintUnset
        );

        let cpi_accounts = Burn {
            mint: ctx.accounts.stake_mint.to_account_info(),
            from: ctx.accounts.voter_ata.to_account_info(),
            authority: ctx.accounts.voter.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts);
        token::burn(cpi_ctx, VOTE_BURN_AMOUNT)?;

        emit!(ReputationVoteBurned {
            voter: ctx.accounts.voter.key(),
            provider_id,
        });
        Ok(())
    }

    /// Set `provider.reputation_score` directly. Authority-gated (the oracle
    /// service holds `config.authority`). Replaces `ccip_mirror_score` in a
    /// Solana-only world — CCIP ingress is no longer relevant.
    pub fn set_reputation_score(
        ctx: Context<SetReputationScore>,
        provider_id: [u8; 32],
        new_score: u16,
    ) -> Result<()> {
        require!(new_score <= 1000, StargazeAnchorError::ScoreOutOfRange);
        require_keys_eq!(
            ctx.accounts.authority.key(),
            ctx.accounts.config.authority,
            StargazeAnchorError::Unauthorized
        );
        let provider = &mut ctx.accounts.provider;
        provider.reputation_score = new_score;
        emit!(ReputationScoreSet {
            provider_id,
            score: new_score,
        });
        Ok(())
    }

    // ============ ESCROW: instructions ============

    /// One-shot initialiser for the escrow side: records the USDC mint and the
    /// router pubkey that is permitted to call `settle`. The caller must be
    /// the existing `Config.authority` admin.
    pub fn init_escrow(
        ctx: Context<InitEscrow>,
        usdc_mint: Pubkey,
        router: Pubkey,
    ) -> Result<()> {
        require_keys_eq!(
            ctx.accounts.admin.key(),
            ctx.accounts.config.authority,
            StargazeAnchorError::Unauthorized
        );
        let escrow = &mut ctx.accounts.usdc_config;
        escrow.admin = ctx.accounts.config.authority;
        escrow.usdc_mint = usdc_mint;
        escrow.router = router;
        escrow.bump = ctx.bumps.usdc_config;

        emit!(EscrowInitialized {
            admin: escrow.admin,
            usdc_mint,
            router,
        });
        Ok(())
    }

    /// Open a new escrow session. The agent deposits `deposit` USDC into the
    /// per-session vault. `spending_limit` caps cumulative settled spend
    /// (must be <= `deposit`). `expires_at` is a unix timestamp after which
    /// settles are rejected and the agent can self-close.
    pub fn open_session(
        ctx: Context<OpenSession>,
        session_id: Pubkey,
        deposit: u64,
        spending_limit: u64,
        expires_at: i64,
    ) -> Result<()> {
        let session_id = session_id.to_bytes();
        require!(deposit > 0, StargazeAnchorError::StakeAmountZero);
        require!(
            spending_limit <= deposit,
            StargazeAnchorError::SpendingLimitExceeded
        );

        let cpi_accounts = Transfer {
            from: ctx.accounts.agent_ata.to_account_info(),
            to: ctx.accounts.session_vault_ata.to_account_info(),
            authority: ctx.accounts.agent.to_account_info(),
        };
        let cpi_ctx =
            CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts);
        token::transfer(cpi_ctx, deposit)?;

        let session = &mut ctx.accounts.session;
        session.session_id = session_id;
        session.agent_wallet = ctx.accounts.agent.key();
        session.deposit = deposit;
        session.spending_limit = spending_limit;
        session.expires_at = expires_at;
        session.settled = false;
        session.total_spent = 0;
        session.total_fee = 0;
        session.bump = ctx.bumps.session;

        emit!(SessionOpened {
            session_id,
            agent_wallet: session.agent_wallet,
            deposit,
            spending_limit,
            expires_at,
        });
        Ok(())
    }

    /// Apply one voucher to the session: transfer (cumulative_amount - prev)
    /// USDC from the session vault, split 98% to the provider ATA and 2% to
    /// the singleton routing-fee vault. The voucher must be paired with an
    /// Ed25519 precompile instruction directly preceding this one in the same
    /// transaction; the program enforces `pubkey == session.agent_wallet`
    /// and the message bytes match the args.
    pub fn settle(
        ctx: Context<Settle>,
        session_id: Pubkey,
        provider_id: Pubkey,
        cumulative_amount: u64,
        nonce: u64,
        message_hash: Pubkey,
    ) -> Result<()> {
        let session_id = session_id.to_bytes();
        let provider_id = provider_id.to_bytes();
        let message_hash = message_hash.to_bytes();
        // Router gate.
        require_keys_eq!(
            ctx.accounts.router.key(),
            ctx.accounts.usdc_config.router,
            StargazeAnchorError::UnauthorizedRouter
        );
        // Read session into a scope-limited block so we can release the
        // immutable borrow before manually mutating cursor + voucher accounts.
        let agent_wallet = ctx.accounts.session.agent_wallet;
        let expires_at = ctx.accounts.session.expires_at;
        let session_settled = ctx.accounts.session.settled;
        let spending_limit = ctx.accounts.session.spending_limit;
        require!(!session_settled, StargazeAnchorError::AlreadySettled);
        let now = Clock::get()?.unix_timestamp;
        require!(now < expires_at, StargazeAnchorError::SessionExpired);

        // Build expected message bytes and assert the preceding Ed25519
        // precompile instruction signed by `session.agent_wallet` carried
        // exactly these bytes.
        let expected_message = build_voucher_message_bytes(
            &session_id,
            &agent_wallet,
            &provider_id,
            cumulative_amount,
            nonce,
        );
        // Recompute the message hash and assert it matches the one used as
        // the `consumed_voucher` PDA seed. This pins replay protection to the
        // canonical voucher bytes — no way to game the seed space.
        let computed_hash = anchor_lang::solana_program::hash::hashv(&[expected_message.as_ref()]);
        require!(
            computed_hash.to_bytes() == message_hash,
            StargazeAnchorError::WrongMessage
        );
        ed25519_verify::verify_preceding_ix(
            &ctx.accounts.instructions_sysvar,
            &agent_wallet,
            &expected_message,
        )?;

        // Manually create voucher_cursor PDA on first use, and consumed_voucher
        // PDA always. Both pay rent from `router`.
        let cursor_bump = ctx.bumps.voucher_cursor;
        let cursor_data_len: u64 = (8 + VoucherCursor::INIT_SPACE) as u64;
        let mut last_cumulative: u64 = 0;
        if ctx.accounts.voucher_cursor.data_is_empty() {
            // Create cursor: system_program::create_account
            let cursor_seeds: &[&[u8]] = &[
                b"voucher_cursor",
                session_id.as_ref(),
                provider_id.as_ref(),
                &[cursor_bump],
            ];
            let signer_seeds = &[cursor_seeds];
            let rent = Rent::get()?.minimum_balance(cursor_data_len as usize);
            let ix = anchor_lang::solana_program::system_instruction::create_account(
                &ctx.accounts.router.key(),
                &ctx.accounts.voucher_cursor.key(),
                rent,
                cursor_data_len,
                &crate::ID,
            );
            anchor_lang::solana_program::program::invoke_signed(
                &ix,
                &[
                    ctx.accounts.router.to_account_info(),
                    ctx.accounts.voucher_cursor.to_account_info(),
                    ctx.accounts.system_program.to_account_info(),
                ],
                signer_seeds,
            )?;
            // Write 8-byte discriminator for VoucherCursor.
            let mut data = ctx.accounts.voucher_cursor.try_borrow_mut_data()?;
            data[..8].copy_from_slice(VoucherCursor::DISCRIMINATOR);
            // Zero-init the rest (last_cumulative=0, bump=cursor_bump).
            data[8..16].copy_from_slice(&0u64.to_le_bytes());
            data[16] = cursor_bump;
        } else {
            // Read last_cumulative from existing cursor.
            let data = ctx.accounts.voucher_cursor.try_borrow_data()?;
            require!(
                data.len() >= 17 && &data[..8] == VoucherCursor::DISCRIMINATOR,
                StargazeAnchorError::SessionAccountMismatch
            );
            last_cumulative = u64::from_le_bytes(data[8..16].try_into().unwrap());
        }

        require!(
            cumulative_amount > last_cumulative,
            StargazeAnchorError::NonMonotonic
        );
        require!(
            cumulative_amount <= spending_limit,
            StargazeAnchorError::SpendingLimitExceeded
        );

        // Create consumed_voucher PDA (always; replay protection).
        let voucher_bump = ctx.bumps.consumed_voucher;
        let voucher_data_len: u64 = (8 + ConsumedVoucher::INIT_SPACE) as u64;
        require!(
            ctx.accounts.consumed_voucher.data_is_empty(),
            StargazeAnchorError::AlreadySettled
        );
        let voucher_seeds: &[&[u8]] = &[
            b"voucher",
            session_id.as_ref(),
            message_hash.as_ref(),
            &[voucher_bump],
        ];
        let signer_seeds = &[voucher_seeds];
        let voucher_rent = Rent::get()?.minimum_balance(voucher_data_len as usize);
        let ix = anchor_lang::solana_program::system_instruction::create_account(
            &ctx.accounts.router.key(),
            &ctx.accounts.consumed_voucher.key(),
            voucher_rent,
            voucher_data_len,
            &crate::ID,
        );
        anchor_lang::solana_program::program::invoke_signed(
            &ix,
            &[
                ctx.accounts.router.to_account_info(),
                ctx.accounts.consumed_voucher.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
            signer_seeds,
        )?;
        {
            let mut data = ctx.accounts.consumed_voucher.try_borrow_mut_data()?;
            data[..8].copy_from_slice(ConsumedVoucher::DISCRIMINATOR);
            data[8] = voucher_bump;
        }

        let delta = cumulative_amount
            .checked_sub(last_cumulative)
            .ok_or(StargazeAnchorError::NumericalOverflow)?;
        let fee = mul_bps(delta, ROUTING_FEE_BPS)?;
        let to_provider = delta
            .checked_sub(fee)
            .ok_or(StargazeAnchorError::NumericalOverflow)?;

        // PDA seeds for session_vault_authority signer.
        let vault_bump = ctx.bumps.session_vault_authority;
        let seeds: &[&[u8]] = &[b"session_vault", session_id.as_ref(), &[vault_bump]];
        let signer_seeds = &[seeds];

        if to_provider > 0 {
            let cpi_accounts = Transfer {
                from: ctx.accounts.session_vault_ata.to_account_info(),
                to: ctx.accounts.provider_ata.to_account_info(),
                authority: ctx.accounts.session_vault_authority.to_account_info(),
            };
            let cpi_ctx = CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                cpi_accounts,
                signer_seeds,
            );
            token::transfer(cpi_ctx, to_provider)?;
        }

        if fee > 0 {
            let cpi_accounts = Transfer {
                from: ctx.accounts.session_vault_ata.to_account_info(),
                to: ctx.accounts.routing_fee_vault_ata.to_account_info(),
                authority: ctx.accounts.session_vault_authority.to_account_info(),
            };
            let cpi_ctx = CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                cpi_accounts,
                signer_seeds,
            );
            token::transfer(cpi_ctx, fee)?;
        }

        // Update cursor + session accumulators.
        {
            let mut cursor_data = ctx.accounts.voucher_cursor.try_borrow_mut_data()?;
            cursor_data[8..16].copy_from_slice(&cumulative_amount.to_le_bytes());
            // bump unchanged
        }
        let session = &mut ctx.accounts.session;
        session.total_spent = session
            .total_spent
            .checked_add(to_provider)
            .ok_or(StargazeAnchorError::NumericalOverflow)?;
        session.total_fee = session
            .total_fee
            .checked_add(fee)
            .ok_or(StargazeAnchorError::NumericalOverflow)?;

        emit!(VoucherSettled {
            session_id,
            provider_id,
            cumulative_amount,
            delta,
            to_provider,
            fee,
            nonce,
        });
        Ok(())
    }

    /// Close the session: pay any remaining deposit back to the agent.
    /// Callable by the router at any time; callable by the agent only after
    /// `expires_at`. The Session account itself is not closed — kept on-chain
    /// for the indexer.
    pub fn close_session(
        ctx: Context<CloseSession>,
        session_id: Pubkey,
    ) -> Result<()> {
        let session_id = session_id.to_bytes();
        let session = &mut ctx.accounts.session;
        require!(!session.settled, StargazeAnchorError::AlreadySettled);

        let caller = ctx.accounts.caller.key();
        let is_router = caller == ctx.accounts.usdc_config.router;
        let is_agent = caller == session.agent_wallet;
        require!(is_router || is_agent, StargazeAnchorError::Unauthorized);

        if is_agent && !is_router {
            let now = Clock::get()?.unix_timestamp;
            require!(
                now >= session.expires_at,
                StargazeAnchorError::SessionNotExpired
            );
        }

        let spent_plus_fee = session
            .total_spent
            .checked_add(session.total_fee)
            .ok_or(StargazeAnchorError::NumericalOverflow)?;
        let refund = session
            .deposit
            .checked_sub(spent_plus_fee)
            .ok_or(StargazeAnchorError::NumericalOverflow)?;

        if refund > 0 {
            let vault_bump = ctx.bumps.session_vault_authority;
            let seeds: &[&[u8]] = &[b"session_vault", session_id.as_ref(), &[vault_bump]];
            let signer_seeds = &[seeds];
            let cpi_accounts = Transfer {
                from: ctx.accounts.session_vault_ata.to_account_info(),
                to: ctx.accounts.agent_ata.to_account_info(),
                authority: ctx.accounts.session_vault_authority.to_account_info(),
            };
            let cpi_ctx = CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                cpi_accounts,
                signer_seeds,
            );
            token::transfer(cpi_ctx, refund)?;
        }

        session.settled = true;

        emit!(SessionSettled {
            session_id,
            total_to_providers: session.total_spent,
            routing_fee: session.total_fee,
            refund_to_agent: refund,
        });
        Ok(())
    }

    // ============ VAULT REGISTRY: instructions ============

    /// Configure (or re-configure) the privacy-vault entry for `provider_id`.
    /// The caller must be `provider.owner`. On re-configure, the existing
    /// `auditor_key` and `buyer_key_rotation_cid` are preserved; only `tier`,
    /// `on_chain_verifier`, `arweave_cid`, and `active` (forced back to true)
    /// are overwritten. Mirrors `PrivacyVaultRegistry.configure` on EVM.
    pub fn configure_vault(
        ctx: Context<ConfigureVault>,
        provider_id: [u8; 32],
        tier: VaultTier,
        on_chain_verifier: Pubkey,
        arweave_cid: [u8; 32],
    ) -> Result<()> {
        require_keys_eq!(
            ctx.accounts.owner.key(),
            ctx.accounts.provider.owner,
            StargazeAnchorError::NotProviderOwner
        );

        let vault = &mut ctx.accounts.vault_config;
        // Preserve auditor_key + buyer_key_rotation_cid across re-configures.
        // First-time init leaves them as the zero defaults Anchor wrote.
        vault.provider_id = provider_id;
        vault.tier = tier;
        vault.on_chain_verifier = on_chain_verifier;
        vault.arweave_cid = arweave_cid;
        vault.active = true;
        vault.bump = ctx.bumps.vault_config;

        emit!(VaultConfigured {
            provider_id,
            tier,
            on_chain_verifier,
            arweave_cid,
        });
        Ok(())
    }

    /// Set the optional auditor key on an active vault. Caller must be
    /// `provider.owner`. Emits the previous + new value so observers can
    /// reconstruct rotation history.
    pub fn set_vault_auditor_key(
        ctx: Context<MutateVault>,
        provider_id: [u8; 32],
        auditor_key: Pubkey,
    ) -> Result<()> {
        require_keys_eq!(
            ctx.accounts.owner.key(),
            ctx.accounts.provider.owner,
            StargazeAnchorError::NotProviderOwner
        );
        let vault = &mut ctx.accounts.vault_config;
        require!(vault.active, StargazeAnchorError::VaultInactive);
        let previous = vault.auditor_key;
        vault.auditor_key = auditor_key;

        emit!(VaultAuditorKeySet {
            provider_id,
            previous,
            current: auditor_key,
        });
        Ok(())
    }

    /// Update the Arweave CID of the per-buyer key rotation policy. Caller
    /// must be `provider.owner`; the vault must be active.
    pub fn set_vault_buyer_key_rotation_cid(
        ctx: Context<MutateVault>,
        provider_id: [u8; 32],
        cid: [u8; 32],
    ) -> Result<()> {
        require_keys_eq!(
            ctx.accounts.owner.key(),
            ctx.accounts.provider.owner,
            StargazeAnchorError::NotProviderOwner
        );
        let vault = &mut ctx.accounts.vault_config;
        require!(vault.active, StargazeAnchorError::VaultInactive);
        vault.buyer_key_rotation_cid = cid;

        emit!(VaultBuyerKeyRotationUpdated { provider_id, cid });
        Ok(())
    }

    /// Admin-only deactivation. Mirrors EVM's `DEFAULT_ADMIN_ROLE`-gated
    /// `deactivate`: the caller must be `Config.authority`, NOT the provider
    /// owner. Sets `active = false`; re-configuring later flips it back.
    pub fn deactivate_vault(
        ctx: Context<DeactivateVault>,
        provider_id: [u8; 32],
    ) -> Result<()> {
        require_keys_eq!(
            ctx.accounts.admin.key(),
            ctx.accounts.config.authority,
            StargazeAnchorError::Unauthorized
        );
        let vault = &mut ctx.accounts.vault_config;
        require!(vault.active, StargazeAnchorError::VaultInactive);
        vault.active = false;

        emit!(VaultDeactivated { provider_id });
        Ok(())
    }

    // ============ VAULT PROOF: instructions ============

    /// Submit a Groth16 proof for `provider_id` against the per-provider
    /// verifier program registered in `VaultConfig.on_chain_verifier`.
    ///
    /// The handler enforces:
    ///   1. `signals_hash == sha256(public_signals)` so the PDA seed truly
    ///      commits to the proof's public inputs (preventing the caller from
    ///      bypassing replay protection by mis-labelling the same proof).
    ///   2. Vault is active and its tier requires a proof (Open is rejected).
    ///   3. The vault has a non-default verifier program id configured.
    ///   4. The passed `verifier_program` account matches the configured id.
    ///   5. The verifier program CPI succeeds.
    ///   6. The `[b"vault_proof", provider_id, signals_hash]` PDA does not
    ///      already exist — `init` (not `init_if_needed`) gives us a single-
    ///      use replay guard "for free".
    ///
    /// Verifier programs are stand-alone Anchor programs that aren't pulled
    /// in as deps; their ix data is constructed manually as
    ///   `[discriminator(8) | borsh(VerifyArgs { proof_bytes, public_signals })]`.
    pub fn submit_vault_proof(
        ctx: Context<SubmitVaultProof>,
        provider_id: [u8; 32],
        signals_hash: [u8; 32],
        proof_bytes: [u8; 256],
        public_signals: Vec<[u8; 32]>,
    ) -> Result<()> {
        let signal_slices: Vec<&[u8]> = public_signals.iter().map(|s| s.as_slice()).collect();
        let computed_hash = anchor_lang::solana_program::hash::hashv(&signal_slices).to_bytes();
        require!(
            computed_hash == signals_hash,
            StargazeAnchorError::SignalsHashMismatch
        );

        let vault = &ctx.accounts.vault_config;
        require!(vault.active, StargazeAnchorError::VaultInactive);
        require!(
            vault.tier != VaultTier::Open,
            StargazeAnchorError::TierDoesNotRequireProof
        );
        require!(
            vault.on_chain_verifier != Pubkey::default(),
            StargazeAnchorError::VerifierUnset
        );
        require_keys_eq!(
            ctx.accounts.verifier_program.key(),
            vault.on_chain_verifier,
            StargazeAnchorError::VerifierProgramMismatch
        );

        // Build the verifier-program ix data: 8-byte Anchor `global:verify`
        // discriminator followed by Borsh(`proof_bytes` || `public_signals`).
        let mut data = Vec::with_capacity(8 + 256 + 4 + public_signals.len() * 32);
        data.extend_from_slice(&VERIFY_IX_DISCRIMINATOR);
        AnchorSerialize::serialize(&proof_bytes, &mut data)?;
        AnchorSerialize::serialize(&public_signals, &mut data)?;

        let cpi_ix = anchor_lang::solana_program::instruction::Instruction {
            program_id: ctx.accounts.verifier_program.key(),
            accounts: vec![],
            data,
        };
        anchor_lang::solana_program::program::invoke(
            &cpi_ix,
            &[ctx.accounts.verifier_program.to_account_info()],
        )
        .map_err(|_| error!(StargazeAnchorError::ProofVerificationFailed))?;

        let tier = vault.tier;
        let record = &mut ctx.accounts.proof_record;
        record.provider_id = provider_id;
        record.signals_hash = signals_hash;
        record.submitter = ctx.accounts.submitter.key();
        record.slot = Clock::get()?.slot;
        record.bump = ctx.bumps.proof_record;

        emit!(VaultProofVerified {
            provider_id,
            tier,
            signals_hash,
            submitter: record.submitter,
            slot: record.slot,
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
pub struct SetReputationScore<'info> {
    pub authority: Signer<'info>,
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
#[instruction(provider_id: [u8; 32], owner: Pubkey)]
pub struct DispatchStakeToTempo<'info> {
    pub sender: Signer<'info>,
    #[account(seeds = [b"staking_config"], bump = staking_config.bump)]
    pub staking_config: Account<'info, StakingConfig>,
    #[account(
        seeds = [b"stake", provider_id.as_ref(), owner.as_ref()],
        bump = stake_account.bump
    )]
    pub stake_account: Account<'info, StakeAccount>,
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

#[derive(Accounts)]
pub struct ProcessRoutingFeeBurn<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        mut,
        seeds = [b"staking_config"],
        bump = staking_config.bump
    )]
    pub staking_config: Account<'info, StakingConfig>,
    #[account(
        mut,
        constraint = stake_mint.key() == staking_config.stake_mint @ StargazeAnchorError::StakeMintUnset
    )]
    pub stake_mint: Account<'info, Mint>,
    #[account(
        mut,
        constraint = authority_ata.owner == authority.key() @ StargazeAnchorError::Unauthorized,
        constraint = authority_ata.mint == stake_mint.key() @ StargazeAnchorError::StakeMintUnset
    )]
    pub authority_ata: Account<'info, TokenAccount>,
    /// CHECK: PDA signer for the staker reward pool ATA. Address is verified
    /// via the seeds constraint; the ATA constraint below pins ownership.
    #[account(seeds = [b"staker_reward_pool"], bump)]
    pub staker_reward_pool_authority: UncheckedAccount<'info>,
    #[account(
        init_if_needed,
        payer = authority,
        associated_token::mint = stake_mint,
        associated_token::authority = staker_reward_pool_authority,
    )]
    pub staker_reward_pool_ata: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(provider_id: [u8; 32])]
pub struct ReputationVoteBurn<'info> {
    pub voter: Signer<'info>,
    #[account(seeds = [b"staking_config"], bump = staking_config.bump)]
    pub staking_config: Account<'info, StakingConfig>,
    #[account(
        mut,
        constraint = stake_mint.key() == staking_config.stake_mint @ StargazeAnchorError::StakeMintUnset
    )]
    pub stake_mint: Account<'info, Mint>,
    #[account(
        mut,
        constraint = voter_ata.owner == voter.key() @ StargazeAnchorError::Unauthorized,
        constraint = voter_ata.mint == stake_mint.key() @ StargazeAnchorError::StakeMintUnset
    )]
    pub voter_ata: Account<'info, TokenAccount>,
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
    /// Cumulative base units burned through `process_routing_fee_burn`. The
    /// reward-pool counterpart lives in `total_routing_fee_to_stakers`.
    pub total_routing_fee_burned: u64,
    /// Cumulative base units routed to the staker reward pool ATA. Pure
    /// accumulator — distribution mechanism is deferred.
    pub total_routing_fee_to_stakers: u64,
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
pub struct ReputationScoreSet {
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
pub struct StakeDispatched {
    pub provider_id: [u8; 32],
    pub owner: Pubkey,
    pub amount: u64,
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

#[event]
pub struct RoutingFeeProcessed {
    pub burned: u64,
    pub to_stakers: u64,
}

#[event]
pub struct ReputationVoteBurned {
    pub voter: Pubkey,
    pub provider_id: [u8; 32],
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
    // ============ ESCROW errors ============
    #[msg("Caller is not the configured router.")]
    UnauthorizedRouter,
    #[msg("Voucher signer does not match the session's agent wallet.")]
    WrongSigner,
    #[msg("Voucher message bytes in the precompile do not match the instruction args.")]
    WrongMessage,
    #[msg("Missing or malformed Ed25519 precompile instruction directly before settle.")]
    MissingPrecompile,
    #[msg("Voucher cumulative amount is not strictly greater than the previous value.")]
    NonMonotonic,
    #[msg("Cumulative spend would exceed the session's spending limit.")]
    SpendingLimitExceeded,
    #[msg("Session has already been settled / closed.")]
    AlreadySettled,
    #[msg("Session expiry has passed.")]
    SessionExpired,
    #[msg("Session expiry has not yet passed; caller must be the router.")]
    SessionNotExpired,
    #[msg("Numerical overflow.")]
    NumericalOverflow,
    #[msg("Session account or vault authority mismatch.")]
    SessionAccountMismatch,
    // ============ VAULT REGISTRY errors ============
    #[msg("Caller is not the registered provider owner.")]
    NotProviderOwner,
    #[msg("Vault PDA does not exist or has been deactivated.")]
    VaultInactive,
    // ============ VAULT PROOF errors ============
    #[msg("Vault has no on-chain verifier configured (Pubkey::default()).")]
    VerifierUnset,
    #[msg("Passed verifier program does not match VaultConfig.on_chain_verifier.")]
    VerifierProgramMismatch,
    #[msg("Verifier program CPI rejected the proof.")]
    ProofVerificationFailed,
    #[msg("Vault tier does not require a Groth16 proof.")]
    TierDoesNotRequireProof,
    #[msg("signals_hash does not commit to sha256 of the passed public_signals.")]
    SignalsHashMismatch,
}

// ============ ESCROW: accounts ============

#[derive(Accounts)]
pub struct InitEscrow<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(seeds = [b"config"], bump = config.bump)]
    pub config: Account<'info, Config>,
    #[account(
        init,
        payer = admin,
        space = 8 + UsdcConfig::INIT_SPACE,
        seeds = [b"usdc_config"],
        bump
    )]
    pub usdc_config: Account<'info, UsdcConfig>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(session_id: Pubkey)]
pub struct OpenSession<'info> {
    #[account(mut)]
    pub agent: Signer<'info>,
    #[account(seeds = [b"usdc_config"], bump = usdc_config.bump)]
    pub usdc_config: Account<'info, UsdcConfig>,
    #[account(
        init,
        payer = agent,
        space = 8 + Session::INIT_SPACE,
        seeds = [b"session", session_id.as_ref()],
        bump
    )]
    pub session: Account<'info, Session>,
    /// CHECK: PDA authority for the per-session USDC vault. Seeds-verified.
    #[account(
        seeds = [b"session_vault", session_id.as_ref()],
        bump
    )]
    pub session_vault_authority: UncheckedAccount<'info>,
    #[account(
        constraint = usdc_mint.key() == usdc_config.usdc_mint @ StargazeAnchorError::SessionAccountMismatch
    )]
    pub usdc_mint: Account<'info, Mint>,
    #[account(
        init_if_needed,
        payer = agent,
        associated_token::mint = usdc_mint,
        associated_token::authority = session_vault_authority,
    )]
    pub session_vault_ata: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = agent_ata.owner == agent.key() @ StargazeAnchorError::Unauthorized,
        constraint = agent_ata.mint == usdc_mint.key() @ StargazeAnchorError::SessionAccountMismatch
    )]
    pub agent_ata: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(
    session_id: Pubkey,
    provider_id: Pubkey,
    cumulative_amount: u64,
    nonce: u64,
    message_hash: Pubkey
)]
pub struct Settle<'info> {
    /// The router pays rent for the `consumed_voucher` PDA. Treated as
    /// protocol overhead.
    #[account(mut)]
    pub router: Signer<'info>,
    #[account(seeds = [b"usdc_config"], bump = usdc_config.bump)]
    pub usdc_config: Box<Account<'info, UsdcConfig>>,
    #[account(
        mut,
        seeds = [b"session", session_id.as_ref()],
        bump = session.bump
    )]
    pub session: Box<Account<'info, Session>>,
    /// CHECK: PDA authority for the per-session USDC vault. Seeds-verified.
    #[account(
        seeds = [b"session_vault", session_id.as_ref()],
        bump
    )]
    pub session_vault_authority: UncheckedAccount<'info>,
    #[account(
        mut,
        constraint = session_vault_ata.owner == session_vault_authority.key() @ StargazeAnchorError::SessionAccountMismatch,
        constraint = session_vault_ata.mint == usdc_config.usdc_mint @ StargazeAnchorError::SessionAccountMismatch
    )]
    pub session_vault_ata: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        constraint = provider_ata.mint == usdc_config.usdc_mint @ StargazeAnchorError::SessionAccountMismatch
    )]
    pub provider_ata: Box<Account<'info, TokenAccount>>,
    /// CHECK: PDA authority for the singleton routing-fee USDC vault.
    #[account(
        seeds = [b"routing_fee_vault"],
        bump
    )]
    pub routing_fee_vault_authority: UncheckedAccount<'info>,
    #[account(
        constraint = usdc_mint.key() == usdc_config.usdc_mint @ StargazeAnchorError::SessionAccountMismatch
    )]
    pub usdc_mint: Box<Account<'info, Mint>>,
    #[account(
        init_if_needed,
        payer = router,
        associated_token::mint = usdc_mint,
        associated_token::authority = routing_fee_vault_authority,
    )]
    pub routing_fee_vault_ata: Box<Account<'info, TokenAccount>>,
    /// CHECK: address verified via seeds; created lazily by the program via
    /// system_program CPI on first use.
    #[account(
        mut,
        seeds = [b"voucher_cursor", session_id.as_ref(), provider_id.as_ref()],
        bump
    )]
    pub voucher_cursor: UncheckedAccount<'info>,
    /// Replay-protection marker, init-created per (session_id, message_hash)
    /// — second use of the same voucher hits `AccountAlreadyInUse` here.
    /// `message_hash` is an instruction arg that the handler asserts equals
    /// `sha256(build_voucher_message_bytes(...))`, so replay protection still
    /// keys off the canonical voucher bytes.
    /// CHECK: address is verified via seeds; init handled manually.
    #[account(
        mut,
        seeds = [b"voucher", session_id.as_ref(), message_hash.as_ref()],
        bump
    )]
    pub consumed_voucher: UncheckedAccount<'info>,
    /// CHECK: Address-pinned to the instructions sysvar; read by `ed25519_verify`.
    #[account(address = anchor_lang::solana_program::sysvar::instructions::ID)]
    pub instructions_sysvar: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(session_id: Pubkey)]
pub struct CloseSession<'info> {
    #[account(mut)]
    pub caller: Signer<'info>,
    #[account(seeds = [b"usdc_config"], bump = usdc_config.bump)]
    pub usdc_config: Account<'info, UsdcConfig>,
    #[account(
        mut,
        seeds = [b"session", session_id.as_ref()],
        bump = session.bump
    )]
    pub session: Account<'info, Session>,
    /// CHECK: PDA authority for the per-session USDC vault. Seeds-verified.
    #[account(
        seeds = [b"session_vault", session_id.as_ref()],
        bump
    )]
    pub session_vault_authority: UncheckedAccount<'info>,
    #[account(
        mut,
        constraint = session_vault_ata.owner == session_vault_authority.key() @ StargazeAnchorError::SessionAccountMismatch,
        constraint = session_vault_ata.mint == usdc_config.usdc_mint @ StargazeAnchorError::SessionAccountMismatch
    )]
    pub session_vault_ata: Account<'info, TokenAccount>,
    /// CHECK: The agent's USDC ATA. Mint+owner verified against the session
    /// record.
    #[account(
        mut,
        constraint = agent_ata.owner == session.agent_wallet @ StargazeAnchorError::SessionAccountMismatch,
        constraint = agent_ata.mint == usdc_config.usdc_mint @ StargazeAnchorError::SessionAccountMismatch
    )]
    pub agent_ata: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}

// ============ ESCROW: state ============

#[account]
#[derive(InitSpace)]
pub struct UsdcConfig {
    pub admin: Pubkey,
    pub usdc_mint: Pubkey,
    pub router: Pubkey,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct Session {
    pub session_id: [u8; 32],
    pub agent_wallet: Pubkey,
    pub deposit: u64,
    pub spending_limit: u64,
    pub expires_at: i64,
    pub settled: bool,
    pub total_spent: u64,
    pub total_fee: u64,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct VoucherCursor {
    pub last_cumulative: u64,
    pub bump: u8,
}

/// PDA marker for replay protection. Anchor refuses to re-`init` a duplicate.
/// Stores the seed bump so re-derivation is cheap if/when we need it; the
/// `last_cumulative` etc lives on the `VoucherCursor`.
#[account]
#[derive(InitSpace)]
pub struct ConsumedVoucher {
    pub bump: u8,
}

// ============ ESCROW: events ============

#[event]
pub struct EscrowInitialized {
    pub admin: Pubkey,
    pub usdc_mint: Pubkey,
    pub router: Pubkey,
}

#[event]
pub struct SessionOpened {
    pub session_id: [u8; 32],
    pub agent_wallet: Pubkey,
    pub deposit: u64,
    pub spending_limit: u64,
    pub expires_at: i64,
}

#[event]
pub struct VoucherSettled {
    pub session_id: [u8; 32],
    pub provider_id: [u8; 32],
    pub cumulative_amount: u64,
    pub delta: u64,
    pub to_provider: u64,
    pub fee: u64,
    pub nonce: u64,
}

#[event]
pub struct SessionSettled {
    pub session_id: [u8; 32],
    pub total_to_providers: u64,
    pub routing_fee: u64,
    pub refund_to_agent: u64,
}

// ============ VAULT REGISTRY: accounts ============

#[derive(Accounts)]
#[instruction(provider_id: [u8; 32])]
pub struct ConfigureVault<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(
        seeds = [b"provider", provider_id.as_ref()],
        bump = provider.bump
    )]
    pub provider: Account<'info, Provider>,
    #[account(
        init_if_needed,
        payer = owner,
        space = 8 + VaultConfig::INIT_SPACE,
        seeds = [b"vault", provider_id.as_ref()],
        bump
    )]
    pub vault_config: Account<'info, VaultConfig>,
    pub system_program: Program<'info, System>,
}

/// Shared accounts context for the two owner-gated mutators (`set_vault_*`).
/// Vault PDA is loaded as a strict `Account<VaultConfig>` so a missing
/// account surfaces as Anchor's `AccountNotInitialized` rather than a
/// custom `VaultInactive` error.
#[derive(Accounts)]
#[instruction(provider_id: [u8; 32])]
pub struct MutateVault<'info> {
    pub owner: Signer<'info>,
    #[account(
        seeds = [b"provider", provider_id.as_ref()],
        bump = provider.bump
    )]
    pub provider: Account<'info, Provider>,
    #[account(
        mut,
        seeds = [b"vault", provider_id.as_ref()],
        bump = vault_config.bump
    )]
    pub vault_config: Account<'info, VaultConfig>,
}

#[derive(Accounts)]
#[instruction(provider_id: [u8; 32])]
pub struct DeactivateVault<'info> {
    pub admin: Signer<'info>,
    #[account(seeds = [b"config"], bump = config.bump)]
    pub config: Account<'info, Config>,
    #[account(
        mut,
        seeds = [b"vault", provider_id.as_ref()],
        bump = vault_config.bump
    )]
    pub vault_config: Account<'info, VaultConfig>,
}

// ============ VAULT REGISTRY: state ============

/// Privacy-tier enum for `VaultConfig`. Mirrors the four EVM tier hashes
/// (`keccak256("open" | "zk-aggregate" | "confidential" | "buyer-key")`) but
/// as a type-safe `repr(u8)` enum — Borsh refuses to deserialize an unknown
/// variant, so the on-chain handler does not need an `UnknownTier` revert.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, InitSpace, Debug)]
#[repr(u8)]
pub enum VaultTier {
    Open = 0,
    ZkAggregate = 1,
    Confidential = 2,
    BuyerKey = 3,
}

#[account]
#[derive(InitSpace)]
pub struct VaultConfig {
    pub provider_id: [u8; 32],
    pub tier: VaultTier,
    /// Solana program id of the per-provider Groth16 verifier. `Pubkey::default()`
    /// is treated as "unset".
    pub on_chain_verifier: Pubkey,
    pub arweave_cid: [u8; 32],
    pub buyer_key_rotation_cid: [u8; 32],
    /// Optional confidential-payments auditor key. `Pubkey::default()` = unset.
    pub auditor_key: Pubkey,
    pub active: bool,
    pub bump: u8,
}

// ============ VAULT REGISTRY: events ============

#[event]
pub struct VaultConfigured {
    pub provider_id: [u8; 32],
    pub tier: VaultTier,
    pub on_chain_verifier: Pubkey,
    pub arweave_cid: [u8; 32],
}

#[event]
pub struct VaultAuditorKeySet {
    pub provider_id: [u8; 32],
    pub previous: Pubkey,
    pub current: Pubkey,
}

#[event]
pub struct VaultBuyerKeyRotationUpdated {
    pub provider_id: [u8; 32],
    pub cid: [u8; 32],
}

#[event]
pub struct VaultDeactivated {
    pub provider_id: [u8; 32],
}

// ============ VAULT PROOF: accounts ============

#[derive(Accounts)]
#[instruction(provider_id: [u8; 32], signals_hash: [u8; 32])]
pub struct SubmitVaultProof<'info> {
    #[account(mut)]
    pub submitter: Signer<'info>,
    #[account(
        seeds = [b"vault", provider_id.as_ref()],
        bump = vault_config.bump
    )]
    pub vault_config: Account<'info, VaultConfig>,
    /// CHECK: the program id is validated against
    /// `vault_config.on_chain_verifier` inside the handler before any CPI.
    /// The account itself is only loaded as the CPI target; stargaze_anchor
    /// never reads or writes its data.
    pub verifier_program: UncheckedAccount<'info>,
    #[account(
        init,
        payer = submitter,
        space = 8 + VaultProofRecord::INIT_SPACE,
        seeds = [b"vault_proof", provider_id.as_ref(), signals_hash.as_ref()],
        bump
    )]
    pub proof_record: Account<'info, VaultProofRecord>,
    pub system_program: Program<'info, System>,
}

// ============ VAULT PROOF: state ============

/// Audit-trail record for a verified Groth16 proof. PDA seeds
/// `[b"vault_proof", provider_id, signals_hash]` are `init`-only, so the
/// account's mere existence is the replay guard.
#[account]
#[derive(InitSpace)]
pub struct VaultProofRecord {
    pub provider_id: [u8; 32],
    pub signals_hash: [u8; 32],
    pub submitter: Pubkey,
    pub slot: u64,
    pub bump: u8,
}

// ============ VAULT PROOF: events ============

#[event]
pub struct VaultProofVerified {
    pub provider_id: [u8; 32],
    pub tier: VaultTier,
    pub signals_hash: [u8; 32],
    pub submitter: Pubkey,
    pub slot: u64,
}

// ============ ESCROW: helpers ============

/// Build the canonical 133-byte voucher message:
///   `b"StargazeMPP/Voucher/1" || session_id || agent_wallet || provider_id
///    || cumulative_amount_le8 || nonce_le8`.
pub fn build_voucher_message_bytes(
    session_id: &[u8; 32],
    agent_wallet: &Pubkey,
    provider_id: &[u8; 32],
    cumulative_amount: u64,
    nonce: u64,
) -> [u8; 133] {
    let mut buf = [0u8; 133];
    buf[..21].copy_from_slice(&VOUCHER_DOMAIN_TAG);
    buf[21..53].copy_from_slice(session_id);
    buf[53..85].copy_from_slice(&agent_wallet.to_bytes());
    buf[85..117].copy_from_slice(provider_id);
    buf[117..125].copy_from_slice(&cumulative_amount.to_le_bytes());
    buf[125..133].copy_from_slice(&nonce.to_le_bytes());
    buf
}

/// Multiply `amount` by `bps` basis points (1 bps = 1/10_000).
fn mul_bps(amount: u64, bps: u16) -> Result<u64> {
    let product = (amount as u128)
        .checked_mul(bps as u128)
        .ok_or(StargazeAnchorError::NumericalOverflow)?;
    let result = product
        .checked_div(10_000)
        .ok_or(StargazeAnchorError::NumericalOverflow)?;
    u64::try_from(result).map_err(|_| StargazeAnchorError::NumericalOverflow.into())
}

/// Parse the Ed25519 precompile instruction directly preceding the current
/// program ix and assert it covers exactly `expected_message` signed by
/// `expected_pubkey`. The single message + single signature + single public
/// key layout is the only one the off-chain SDK emits.
pub mod ed25519_verify {
    use super::*;
    use anchor_lang::solana_program::ed25519_program;
    use anchor_lang::solana_program::sysvar::instructions::{
        load_current_index_checked, load_instruction_at_checked,
    };

    /// Precompile data layout (single-signature mode):
    ///   `[num_signatures: u8 = 1][padding: u8 = 0][offsets: 14 bytes]
    ///    [pubkey: 32][signature: 64][message: variable]`.
    ///
    /// `*_instruction_index` fields inside `offsets` must equal `u16::MAX`
    /// (i.e. "data is inside this same ix"). Offsets are little-endian u16s.
    const NUM_SIGS_OFFSET: usize = 0;
    const OFFSETS_START: usize = 2;
    const SIG_OFFSETS_LEN: usize = 14;
    const DATA_START: usize = OFFSETS_START + SIG_OFFSETS_LEN; // 16
    const PUBKEY_LEN: usize = 32;
    const SIGNATURE_LEN: usize = 64;

    pub fn verify_preceding_ix(
        instructions_sysvar: &UncheckedAccount,
        expected_pubkey: &Pubkey,
        expected_message: &[u8],
    ) -> Result<()> {
        let info: &AccountInfo = instructions_sysvar;
        let current_index = load_current_index_checked(info)
            .map_err(|_| StargazeAnchorError::MissingPrecompile)?;
        require!(current_index > 0, StargazeAnchorError::MissingPrecompile);

        let prev_ix = load_instruction_at_checked((current_index - 1) as usize, info)
            .map_err(|_| StargazeAnchorError::MissingPrecompile)?;
        require_keys_eq!(
            prev_ix.program_id,
            ed25519_program::ID,
            StargazeAnchorError::MissingPrecompile
        );

        let data = &prev_ix.data;
        require!(
            data.len() >= DATA_START + PUBKEY_LEN + SIGNATURE_LEN,
            StargazeAnchorError::MissingPrecompile
        );
        // Exactly one signature.
        require!(
            data[NUM_SIGS_OFFSET] == 1,
            StargazeAnchorError::MissingPrecompile
        );

        let read_u16 = |off: usize| -> u16 {
            u16::from_le_bytes([data[off], data[off + 1]])
        };
        let signature_offset = read_u16(OFFSETS_START) as usize;
        let signature_ix_index = read_u16(OFFSETS_START + 2);
        let public_key_offset = read_u16(OFFSETS_START + 4) as usize;
        let public_key_ix_index = read_u16(OFFSETS_START + 6);
        let message_data_offset = read_u16(OFFSETS_START + 8) as usize;
        let message_data_size = read_u16(OFFSETS_START + 10) as usize;
        let message_ix_index = read_u16(OFFSETS_START + 12);

        // All three must point into the precompile's own ix data.
        require!(
            signature_ix_index == u16::MAX
                && public_key_ix_index == u16::MAX
                && message_ix_index == u16::MAX,
            StargazeAnchorError::MissingPrecompile
        );

        // Sanity-check the offsets land inside the data buffer.
        require!(
            public_key_offset + PUBKEY_LEN <= data.len()
                && signature_offset + SIGNATURE_LEN <= data.len()
                && message_data_offset + message_data_size <= data.len(),
            StargazeAnchorError::MissingPrecompile
        );

        let pubkey_bytes = &data[public_key_offset..public_key_offset + PUBKEY_LEN];
        let message_bytes =
            &data[message_data_offset..message_data_offset + message_data_size];

        let pk_array: [u8; 32] = pubkey_bytes
            .try_into()
            .map_err(|_| StargazeAnchorError::WrongSigner)?;
        let signer_key = Pubkey::new_from_array(pk_array);
        require_keys_eq!(
            signer_key,
            *expected_pubkey,
            StargazeAnchorError::WrongSigner
        );

        // Compare the message bytes by value.
        require!(
            message_bytes == expected_message,
            StargazeAnchorError::WrongMessage
        );

        Ok(())
    }
}
