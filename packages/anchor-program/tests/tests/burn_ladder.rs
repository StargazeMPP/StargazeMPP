//! Integration tests for the routing-fee and reputation-vote burn ladder on
//! `stargaze_anchor`. Exercises both new instructions (`process_routing_fee_burn`
//! and `reputation_vote_burn`) end-to-end through the on-chain SPL Token and
//! Associated Token Account programs, asserting that SPL supply truly drops
//! on burn and that the staker reward pool accumulates correctly.

use anchor_lang::AnchorDeserialize;
use solana_sdk::{
    instruction::Instruction,
    message::Message,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use stargaze_anchor::{
    ReputationVoteBurned, RoutingFeeProcessed, StakingConfig, COOLDOWN_DEFAULT_SECS,
    MIN_STAKE_DEFAULT, VERIFIED_STAKE_DEFAULT, VOTE_BURN_AMOUNT,
};
use stargaze_anchor_tests::{
    associated_token_address, create_associated_token_account, create_mint, find_event,
    ix_init_staking, ix_initialize, ix_process_routing_fee_burn, ix_reputation_vote_burn,
    ix_set_stake_mint, mint_supply, mint_to, setup_svm, staker_reward_pool_authority_pda,
    staking_config_pda, token_balance,
};

const PROVIDER_ID: [u8; 32] = [0xABu8; 32];

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

/// Bring staking up with a real mint, fund the `authority` ATA with
/// `authority_balance`, and return the mint keypair. Mirrors the
/// `bootstrap_with_mint` helper from `staking.rs` but tailored for the burn
/// ladder: the authority — not a staker — is the token holder.
fn bootstrap_with_authority_balance(
    svm: &mut litesvm::LiteSVM,
    authority: &Keypair,
    authority_balance: u64,
) -> (Keypair, Pubkey) {
    send(
        svm,
        authority,
        &[authority],
        &[ix_initialize(&authority.pubkey(), authority.pubkey())],
    )
    .expect("initialize");

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

    let mint_kp = create_mint(svm, authority, &authority.pubkey(), 6);
    let mint = mint_kp.pubkey();

    send(
        svm,
        authority,
        &[authority],
        &[ix_set_stake_mint(&authority.pubkey(), mint)],
    )
    .expect("set_stake_mint");

    let authority_ata =
        create_associated_token_account(svm, authority, &authority.pubkey(), &mint);
    if authority_balance > 0 {
        mint_to(svm, authority, &mint, &authority_ata, authority, authority_balance);
    }

    (mint_kp, authority_ata)
}

#[test]
fn process_routing_fee_burn_splits_50_50() {
    let (mut svm, authority) = setup_svm();
    let initial: u64 = 10_000_000;
    let (mint_kp, authority_ata) =
        bootstrap_with_authority_balance(&mut svm, &authority, initial);
    let mint = mint_kp.pubkey();

    let supply_before = mint_supply(&svm, &mint);
    let amount: u64 = 1_000_000;

    send(
        &mut svm,
        &authority,
        &[&authority],
        &[ix_process_routing_fee_burn(&authority.pubkey(), &mint, amount)],
    )
    .expect("process_routing_fee_burn");

    let burned = amount / 2;
    let to_stakers = amount - burned;

    let (reward_pool_auth, _) = staker_reward_pool_authority_pda();
    let reward_pool_ata = associated_token_address(&reward_pool_auth, &mint);

    assert_eq!(token_balance(&svm, &authority_ata), initial - amount);
    assert_eq!(token_balance(&svm, &reward_pool_ata), to_stakers);
    assert_eq!(mint_supply(&svm, &mint), supply_before - burned);

    let cfg = read_staking_config(&svm);
    assert_eq!(cfg.total_routing_fee_burned, burned);
    assert_eq!(cfg.total_routing_fee_to_stakers, to_stakers);
}

#[test]
fn process_routing_fee_burn_odd_amount_favours_stakers() {
    let (mut svm, authority) = setup_svm();
    let (mint_kp, authority_ata) =
        bootstrap_with_authority_balance(&mut svm, &authority, 100);
    let mint = mint_kp.pubkey();

    let supply_before = mint_supply(&svm, &mint);

    send(
        &mut svm,
        &authority,
        &[&authority],
        &[ix_process_routing_fee_burn(&authority.pubkey(), &mint, 3)],
    )
    .expect("process_routing_fee_burn(amount=3)");

    let (reward_pool_auth, _) = staker_reward_pool_authority_pda();
    let reward_pool_ata = associated_token_address(&reward_pool_auth, &mint);

    // amount=3 → burned=1, to_stakers=2 (odd unit routes to stakers).
    assert_eq!(token_balance(&svm, &authority_ata), 100 - 3);
    assert_eq!(token_balance(&svm, &reward_pool_ata), 2);
    assert_eq!(mint_supply(&svm, &mint), supply_before - 1);

    let cfg = read_staking_config(&svm);
    assert_eq!(cfg.total_routing_fee_burned, 1);
    assert_eq!(cfg.total_routing_fee_to_stakers, 2);
}

#[test]
fn process_routing_fee_burn_rejects_non_admin() {
    let (mut svm, authority) = setup_svm();
    let (mint_kp, _authority_ata) =
        bootstrap_with_authority_balance(&mut svm, &authority, 10_000_000);
    let mint = mint_kp.pubkey();

    let attacker = Keypair::new();
    svm.airdrop(&attacker.pubkey(), 10_000_000_000)
        .expect("airdrop attacker");
    // Give the attacker an ATA + balance so the failure is on auth, not the
    // ATA check.
    let attacker_ata =
        create_associated_token_account(&mut svm, &authority, &attacker.pubkey(), &mint);
    mint_to(&mut svm, &authority, &mint, &attacker_ata, &authority, 1_000_000);

    let err = send(
        &mut svm,
        &attacker,
        &[&attacker],
        &[ix_process_routing_fee_burn(&attacker.pubkey(), &mint, 1_000_000)],
    )
    .expect_err("non-authority caller must be rejected");
    let logs = err.meta.logs.join("\n");
    assert!(
        logs.contains("Unauthorized") || logs.contains("authorised"),
        "expected Unauthorized in logs, got:\n{logs}"
    );
}

#[test]
fn process_routing_fee_burn_accumulates_in_pool() {
    let (mut svm, authority) = setup_svm();
    let initial: u64 = 10_000_000;
    let (mint_kp, _authority_ata) =
        bootstrap_with_authority_balance(&mut svm, &authority, initial);
    let mint = mint_kp.pubkey();

    let supply_before = mint_supply(&svm, &mint);

    send(
        &mut svm,
        &authority,
        &[&authority],
        &[ix_process_routing_fee_burn(&authority.pubkey(), &mint, 1_000_000)],
    )
    .expect("first call");
    send(
        &mut svm,
        &authority,
        &[&authority],
        &[ix_process_routing_fee_burn(&authority.pubkey(), &mint, 2_000_000)],
    )
    .expect("second call");

    let (reward_pool_auth, _) = staker_reward_pool_authority_pda();
    let reward_pool_ata = associated_token_address(&reward_pool_auth, &mint);

    // 1m: burn 500k, to_stakers 500k.
    // 2m: burn 1m,   to_stakers 1m.
    let total_burned: u64 = 1_500_000;
    let total_to_stakers: u64 = 1_500_000;

    assert_eq!(token_balance(&svm, &reward_pool_ata), total_to_stakers);
    assert_eq!(mint_supply(&svm, &mint), supply_before - total_burned);

    let cfg = read_staking_config(&svm);
    assert_eq!(cfg.total_routing_fee_burned, total_burned);
    assert_eq!(cfg.total_routing_fee_to_stakers, total_to_stakers);
}

#[test]
fn process_routing_fee_burn_rejects_zero() {
    let (mut svm, authority) = setup_svm();
    let (mint_kp, _authority_ata) =
        bootstrap_with_authority_balance(&mut svm, &authority, 10_000_000);
    let mint = mint_kp.pubkey();

    let err = send(
        &mut svm,
        &authority,
        &[&authority],
        &[ix_process_routing_fee_burn(&authority.pubkey(), &mint, 0)],
    )
    .expect_err("zero amount must be rejected");
    let logs = err.meta.logs.join("\n");
    assert!(
        logs.contains("StakeAmountZero") || logs.contains("greater than zero"),
        "expected StakeAmountZero in logs, got:\n{logs}"
    );
}

#[test]
fn process_routing_fee_burn_rejects_insufficient_balance() {
    let (mut svm, authority) = setup_svm();
    let (mint_kp, _authority_ata) =
        bootstrap_with_authority_balance(&mut svm, &authority, 100);
    let mint = mint_kp.pubkey();

    // amount=200 against balance=100 — token program must surface an error.
    let err = send(
        &mut svm,
        &authority,
        &[&authority],
        &[ix_process_routing_fee_burn(&authority.pubkey(), &mint, 200)],
    )
    .expect_err("insufficient balance must be rejected");
    // We don't assert a specific error string here — any failure from the
    // SPL Token program is acceptable.
    let _ = err;
}

#[test]
fn reputation_vote_burn_burns_one_token() {
    let (mut svm, authority) = setup_svm();
    // Bootstrap staking — the authority ATA is fine to leave at zero; the
    // voter is a separate account.
    let (mint_kp, _authority_ata) =
        bootstrap_with_authority_balance(&mut svm, &authority, 0);
    let mint = mint_kp.pubkey();

    let voter = Keypair::new();
    svm.airdrop(&voter.pubkey(), 10_000_000_000)
        .expect("airdrop voter");
    let voter_ata =
        create_associated_token_account(&mut svm, &authority, &voter.pubkey(), &mint);
    let voter_initial: u64 = 5_000_000;
    mint_to(&mut svm, &authority, &mint, &voter_ata, &authority, voter_initial);

    let supply_before = mint_supply(&svm, &mint);

    send(
        &mut svm,
        &voter,
        &[&voter],
        &[ix_reputation_vote_burn(&voter.pubkey(), &mint, PROVIDER_ID)],
    )
    .expect("reputation_vote_burn");

    assert_eq!(token_balance(&svm, &voter_ata), voter_initial - VOTE_BURN_AMOUNT);
    assert_eq!(mint_supply(&svm, &mint), supply_before - VOTE_BURN_AMOUNT);
}

#[test]
fn reputation_vote_burn_rejects_insufficient_balance() {
    let (mut svm, authority) = setup_svm();
    let (mint_kp, _authority_ata) =
        bootstrap_with_authority_balance(&mut svm, &authority, 0);
    let mint = mint_kp.pubkey();

    // Voter has an ATA but zero balance.
    let voter = Keypair::new();
    svm.airdrop(&voter.pubkey(), 10_000_000_000)
        .expect("airdrop voter");
    let _voter_ata =
        create_associated_token_account(&mut svm, &authority, &voter.pubkey(), &mint);

    let err = send(
        &mut svm,
        &voter,
        &[&voter],
        &[ix_reputation_vote_burn(&voter.pubkey(), &mint, PROVIDER_ID)],
    )
    .expect_err("zero balance must be rejected");
    let _ = err;
}

#[test]
fn reputation_vote_burn_emits_event() {
    let (mut svm, authority) = setup_svm();
    let (mint_kp, _authority_ata) =
        bootstrap_with_authority_balance(&mut svm, &authority, 0);
    let mint = mint_kp.pubkey();

    let voter = Keypair::new();
    svm.airdrop(&voter.pubkey(), 10_000_000_000)
        .expect("airdrop voter");
    let voter_ata =
        create_associated_token_account(&mut svm, &authority, &voter.pubkey(), &mint);
    mint_to(&mut svm, &authority, &mint, &voter_ata, &authority, VOTE_BURN_AMOUNT);

    let meta = send(
        &mut svm,
        &voter,
        &[&voter],
        &[ix_reputation_vote_burn(&voter.pubkey(), &mint, PROVIDER_ID)],
    )
    .expect("reputation_vote_burn");

    let evt: ReputationVoteBurned =
        find_event(&meta.logs).expect("ReputationVoteBurned event present");
    assert_eq!(evt.voter, voter.pubkey());
    assert_eq!(evt.provider_id, PROVIDER_ID);

    // Also assert the routing-fee event helper resolves (sanity check on
    // the RoutingFeeProcessed type being public/importable).
    let _ = std::mem::size_of::<RoutingFeeProcessed>();
}
