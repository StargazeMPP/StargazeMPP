//! Integration tests for the staking instructions on `stargaze_anchor`.
//!
//! These tests use the in-process `litesvm` runtime and the on-chain SPL
//! Token + Associated Token Account programs to verify the full token-flow
//! path (stake -> request_unstake -> claim_unstake, and the admin `slash`).

use anchor_lang::AnchorDeserialize;
use solana_sdk::{
    instruction::Instruction,
    message::Message,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use stargaze_anchor::{StakeAccount, StakingConfig, COOLDOWN_DEFAULT_SECS, MIN_STAKE_DEFAULT, VERIFIED_STAKE_DEFAULT};
use stargaze_anchor_tests::{
    associated_token_address, create_associated_token_account, create_mint, ensure_system_account,
    ix_claim_unstake, ix_init_staking, ix_initialize, ix_request_unstake, ix_set_stake_mint,
    ix_slash, ix_stake, mint_to, setup_svm, stake_account_pda, stake_pool_authority_pda,
    staking_config_pda, token_balance, warp_clock, BURN_DESTINATION,
};

const PROVIDER_ID: [u8; 32] = [9u8; 32];

fn send(
    svm: &mut litesvm::LiteSVM,
    payer: &Keypair,
    signers: &[&Keypair],
    ixs: &[Instruction],
) -> Result<litesvm::types::TransactionMetadata, litesvm::types::FailedTransactionMetadata> {
    let blockhash = svm.latest_blockhash();
    let msg = Message::new(ixs, Some(&payer.pubkey()));
    let tx = Transaction::new(signers, msg, blockhash);
    svm.send_transaction(tx)
}

fn read_staking_config(svm: &litesvm::LiteSVM) -> StakingConfig {
    let (addr, _) = staking_config_pda();
    let acct = svm.get_account(&addr).expect("staking_config exists");
    let mut data = &acct.data[8..];
    StakingConfig::deserialize(&mut data).expect("decode staking_config")
}

fn read_stake_account(svm: &litesvm::LiteSVM, provider_id: &[u8; 32], owner: &Pubkey) -> StakeAccount {
    let (addr, _) = stake_account_pda(provider_id, owner);
    let acct = svm.get_account(&addr).expect("stake_account exists");
    let mut data = &acct.data[8..];
    StakeAccount::deserialize(&mut data).expect("decode stake_account")
}

/// Bring the staking system up to a usable state with a real mint and seeded
/// staker balance. Returns `(authority, staker, mint, staker_ata)`.
fn bootstrap_with_mint(
    svm: &mut litesvm::LiteSVM,
    authority: &Keypair,
    initial_balance: u64,
) -> (Keypair, Keypair, Pubkey) {
    // Initialise the base config with `authority` as the admin.
    send(
        svm,
        authority,
        &[authority],
        &[ix_initialize(&authority.pubkey(), authority.pubkey())],
    )
    .expect("initialize");

    // Initialise staking config with a *deferred* mint so we can exercise the
    // `set_stake_mint` path that creates the pool ATA.
    send(
        svm,
        authority,
        &[authority],
        &[ix_init_staking(
            &authority.pubkey(),
            Pubkey::default(),
            MIN_STAKE_DEFAULT,
            VERIFIED_STAKE_DEFAULT,
            COOLDOWN_DEFAULT_SECS,
        )],
    )
    .expect("init_staking");

    // Create the SPL mint with `authority` as the mint authority.
    let mint_kp = create_mint(svm, authority, &authority.pubkey(), 6);
    let mint = mint_kp.pubkey();

    // Wire the mint into the staking config; this also creates the pool ATA.
    send(
        svm,
        authority,
        &[authority],
        &[ix_set_stake_mint(&authority.pubkey(), mint)],
    )
    .expect("set_stake_mint");

    // Set up a staker with an ATA and an initial balance.
    let staker = Keypair::new();
    svm.airdrop(&staker.pubkey(), 10_000_000_000)
        .expect("airdrop staker");
    let staker_ata = create_associated_token_account(svm, authority, &staker.pubkey(), &mint);
    mint_to(svm, authority, &mint, &staker_ata, authority, initial_balance);

    (mint_kp, staker, staker_ata)
}

#[test]
fn init_staking_sets_config() {
    let (mut svm, authority) = setup_svm();
    send(
        &mut svm,
        &authority,
        &[&authority],
        &[ix_initialize(&authority.pubkey(), authority.pubkey())],
    )
    .expect("initialize");

    send(
        &mut svm,
        &authority,
        &[&authority],
        &[ix_init_staking(
            &authority.pubkey(),
            Pubkey::default(),
            MIN_STAKE_DEFAULT,
            VERIFIED_STAKE_DEFAULT,
            COOLDOWN_DEFAULT_SECS,
        )],
    )
    .expect("init_staking");

    let cfg = read_staking_config(&svm);
    assert_eq!(cfg.authority, authority.pubkey());
    assert_eq!(cfg.stake_mint, Pubkey::default());
    assert_eq!(cfg.min_stake, MIN_STAKE_DEFAULT);
    assert_eq!(cfg.verified_stake, VERIFIED_STAKE_DEFAULT);
    assert_eq!(cfg.cooldown_secs, COOLDOWN_DEFAULT_SECS);

    // Non-authority caller must be rejected.
    let attacker = Keypair::new();
    svm.airdrop(&attacker.pubkey(), 1_000_000_000)
        .expect("airdrop attacker");
    let err = send(
        &mut svm,
        &attacker,
        &[&attacker],
        &[ix_init_staking(
            &attacker.pubkey(),
            Pubkey::default(),
            MIN_STAKE_DEFAULT,
            VERIFIED_STAKE_DEFAULT,
            COOLDOWN_DEFAULT_SECS,
        )],
    )
    .expect_err("non-authority must be rejected");
    let _ = err;
}

#[test]
fn set_stake_mint_one_shot() {
    let (mut svm, authority) = setup_svm();
    send(
        &mut svm,
        &authority,
        &[&authority],
        &[ix_initialize(&authority.pubkey(), authority.pubkey())],
    )
    .expect("initialize");
    send(
        &mut svm,
        &authority,
        &[&authority],
        &[ix_init_staking(
            &authority.pubkey(),
            Pubkey::default(),
            MIN_STAKE_DEFAULT,
            VERIFIED_STAKE_DEFAULT,
            COOLDOWN_DEFAULT_SECS,
        )],
    )
    .expect("init_staking");

    let mint_kp_a = create_mint(&mut svm, &authority, &authority.pubkey(), 6);
    send(
        &mut svm,
        &authority,
        &[&authority],
        &[ix_set_stake_mint(&authority.pubkey(), mint_kp_a.pubkey())],
    )
    .expect("set_stake_mint first call");

    let cfg = read_staking_config(&svm);
    assert_eq!(cfg.stake_mint, mint_kp_a.pubkey());

    // Pool ATA must exist and be owned by the stake_pool_authority PDA.
    let (pool_auth, _) = stake_pool_authority_pda();
    let pool_ata = associated_token_address(&pool_auth, &mint_kp_a.pubkey());
    let pool_acct = svm.get_account(&pool_ata).expect("pool ATA exists");
    assert!(!pool_acct.data.is_empty());

    // Second call with a different mint must be rejected.
    let mint_kp_b = create_mint(&mut svm, &authority, &authority.pubkey(), 6);
    let err = send(
        &mut svm,
        &authority,
        &[&authority],
        &[ix_set_stake_mint(&authority.pubkey(), mint_kp_b.pubkey())],
    )
    .expect_err("second set_stake_mint must be rejected");
    let _ = err;
}

#[test]
fn stake_transfers_to_pool() {
    let (mut svm, authority) = setup_svm();
    let initial = 5 * MIN_STAKE_DEFAULT;
    let (mint_kp, staker, staker_ata) = bootstrap_with_mint(&mut svm, &authority, initial);
    let mint = mint_kp.pubkey();

    let stake_amount = 2 * MIN_STAKE_DEFAULT;
    send(
        &mut svm,
        &staker,
        &[&staker],
        &[ix_stake(&staker.pubkey(), &mint, PROVIDER_ID, stake_amount)],
    )
    .expect("stake");

    let (pool_auth, _) = stake_pool_authority_pda();
    let pool_ata = associated_token_address(&pool_auth, &mint);
    assert_eq!(token_balance(&svm, &pool_ata), stake_amount);
    assert_eq!(token_balance(&svm, &staker_ata), initial - stake_amount);

    let stake_acct = read_stake_account(&svm, &PROVIDER_ID, &staker.pubkey());
    assert_eq!(stake_acct.amount, stake_amount);
    assert_eq!(stake_acct.cooldown_amount, 0);
    assert_eq!(stake_acct.owner, staker.pubkey());
    assert_eq!(stake_acct.provider_id, PROVIDER_ID);
}

#[test]
fn request_unstake_then_claim() {
    let (mut svm, authority) = setup_svm();
    let initial = 10 * MIN_STAKE_DEFAULT;
    let (mint_kp, staker, staker_ata) = bootstrap_with_mint(&mut svm, &authority, initial);
    let mint = mint_kp.pubkey();

    let stake_amount = 4 * MIN_STAKE_DEFAULT;
    send(
        &mut svm,
        &staker,
        &[&staker],
        &[ix_stake(&staker.pubkey(), &mint, PROVIDER_ID, stake_amount)],
    )
    .expect("stake");

    let unstake_amount = 3 * MIN_STAKE_DEFAULT;
    send(
        &mut svm,
        &staker,
        &[&staker],
        &[ix_request_unstake(&staker.pubkey(), PROVIDER_ID, unstake_amount)],
    )
    .expect("request_unstake");

    let acct_mid = read_stake_account(&svm, &PROVIDER_ID, &staker.pubkey());
    assert_eq!(acct_mid.amount, stake_amount);
    assert_eq!(acct_mid.cooldown_amount, unstake_amount);
    assert!(acct_mid.cooldown_start_ts > 0);

    // Warp past the cooldown window.
    warp_clock(&mut svm, COOLDOWN_DEFAULT_SECS + 1);
    svm.expire_blockhash();

    send(
        &mut svm,
        &staker,
        &[&staker],
        &[ix_claim_unstake(&staker.pubkey(), &mint, PROVIDER_ID)],
    )
    .expect("claim_unstake");

    let acct_after = read_stake_account(&svm, &PROVIDER_ID, &staker.pubkey());
    assert_eq!(acct_after.amount, stake_amount - unstake_amount);
    assert_eq!(acct_after.cooldown_amount, 0);
    assert_eq!(acct_after.cooldown_start_ts, 0);

    let (pool_auth, _) = stake_pool_authority_pda();
    let pool_ata = associated_token_address(&pool_auth, &mint);
    assert_eq!(token_balance(&svm, &pool_ata), stake_amount - unstake_amount);
    assert_eq!(
        token_balance(&svm, &staker_ata),
        initial - stake_amount + unstake_amount,
    );
}

#[test]
fn claim_before_cooldown_fails() {
    let (mut svm, authority) = setup_svm();
    let initial = 4 * MIN_STAKE_DEFAULT;
    let (mint_kp, staker, _staker_ata) = bootstrap_with_mint(&mut svm, &authority, initial);
    let mint = mint_kp.pubkey();

    let stake_amount = 2 * MIN_STAKE_DEFAULT;
    send(
        &mut svm,
        &staker,
        &[&staker],
        &[ix_stake(&staker.pubkey(), &mint, PROVIDER_ID, stake_amount)],
    )
    .expect("stake");

    send(
        &mut svm,
        &staker,
        &[&staker],
        &[ix_request_unstake(&staker.pubkey(), PROVIDER_ID, stake_amount)],
    )
    .expect("request_unstake");

    // Immediate claim — no warp, should revert with CooldownActive.
    let err = send(
        &mut svm,
        &staker,
        &[&staker],
        &[ix_claim_unstake(&staker.pubkey(), &mint, PROVIDER_ID)],
    )
    .expect_err("claim must fail before cooldown elapses");
    let logs = err.meta.logs.join("\n");
    assert!(
        logs.contains("CooldownActive") || logs.contains("0x1796") || logs.contains("Cooldown period"),
        "expected CooldownActive in logs, got:\n{logs}"
    );
}

#[test]
fn slash_transfers_to_burn_address() {
    let (mut svm, authority) = setup_svm();
    let initial = 6 * MIN_STAKE_DEFAULT;
    let (mint_kp, staker, _staker_ata) = bootstrap_with_mint(&mut svm, &authority, initial);
    let mint = mint_kp.pubkey();

    // Make sure the burn destination exists as a system account so the ATA
    // creation against it succeeds.
    ensure_system_account(&mut svm, &BURN_DESTINATION, 1_000_000_000);
    let burn_ata = create_associated_token_account(&mut svm, &authority, &BURN_DESTINATION, &mint);

    let stake_amount = 4 * MIN_STAKE_DEFAULT;
    send(
        &mut svm,
        &staker,
        &[&staker],
        &[ix_stake(&staker.pubkey(), &mint, PROVIDER_ID, stake_amount)],
    )
    .expect("stake");

    let slash_amount = MIN_STAKE_DEFAULT;
    send(
        &mut svm,
        &authority,
        &[&authority],
        &[ix_slash(
            &authority.pubkey(),
            &mint,
            PROVIDER_ID,
            staker.pubkey(),
            slash_amount,
        )],
    )
    .expect("slash");

    let (pool_auth, _) = stake_pool_authority_pda();
    let pool_ata = associated_token_address(&pool_auth, &mint);
    assert_eq!(token_balance(&svm, &pool_ata), stake_amount - slash_amount);
    assert_eq!(token_balance(&svm, &burn_ata), slash_amount);

    let stake_acct = read_stake_account(&svm, &PROVIDER_ID, &staker.pubkey());
    assert_eq!(stake_acct.amount, stake_amount - slash_amount);
}

#[test]
fn slash_caps_at_available_stake() {
    let (mut svm, authority) = setup_svm();
    let initial = 5 * MIN_STAKE_DEFAULT;
    let (mint_kp, staker, _staker_ata) = bootstrap_with_mint(&mut svm, &authority, initial);
    let mint = mint_kp.pubkey();

    ensure_system_account(&mut svm, &BURN_DESTINATION, 1_000_000_000);
    let burn_ata = create_associated_token_account(&mut svm, &authority, &BURN_DESTINATION, &mint);

    let stake_amount = 2 * MIN_STAKE_DEFAULT;
    send(
        &mut svm,
        &staker,
        &[&staker],
        &[ix_stake(&staker.pubkey(), &mint, PROVIDER_ID, stake_amount)],
    )
    .expect("stake");

    // Ask to slash 10x what's there.
    let huge = stake_amount * 10;
    send(
        &mut svm,
        &authority,
        &[&authority],
        &[ix_slash(
            &authority.pubkey(),
            &mint,
            PROVIDER_ID,
            staker.pubkey(),
            huge,
        )],
    )
    .expect("slash (capped)");

    let (pool_auth, _) = stake_pool_authority_pda();
    let pool_ata = associated_token_address(&pool_auth, &mint);
    assert_eq!(token_balance(&svm, &pool_ata), 0);
    assert_eq!(token_balance(&svm, &burn_ata), stake_amount);

    let stake_acct = read_stake_account(&svm, &PROVIDER_ID, &staker.pubkey());
    assert_eq!(stake_acct.amount, 0);
}
