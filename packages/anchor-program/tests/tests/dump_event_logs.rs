//! Captures the exact `Program data: …` log lines that
//! `stargaze_anchor`'s `emit!` macro produces, so the indexer crate can
//! pin its decoder against real on-chain output. Run with
//! `cargo test --test dump_event_logs -- --nocapture` and paste the
//! resulting fixture into the indexer's fixture test.

use solana_sdk::{
    instruction::Instruction,
    message::Message,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use stargaze_anchor::{
    COOLDOWN_DEFAULT_SECS, MIN_STAKE_DEFAULT, VERIFIED_STAKE_DEFAULT, VOTE_BURN_AMOUNT,
};
use stargaze_anchor_tests::{
    create_associated_token_account, create_mint, ensure_system_account,
    ix_claim_unstake, ix_dispatch_reputation_to_tempo, ix_dispatch_stake_to_tempo,
    ix_init_staking, ix_initialize, ix_process_routing_fee_burn, ix_register_provider,
    ix_reputation_vote_burn, ix_request_unstake, ix_set_stake_mint, ix_slash, ix_stake,
    mint_to, setup_svm, warp_clock, BURN_DESTINATION,
};

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

fn dump(label: &str, logs: &[String]) {
    for line in logs {
        if line.starts_with("Program data: ") {
            println!("{label} {line}");
        }
    }
}

#[test]
fn dumps_register_and_ccip_dispatch_log_lines() {
    let (mut svm, authority) = setup_svm();
    let provider_id = [42u8; 32];

    let m1 = send(
        &mut svm,
        &authority,
        &[&authority],
        &[ix_initialize(&authority.pubkey(), authority.pubkey())],
    )
    .expect("initialize");
    dump("initialize", &m1.logs);

    let m2 = send(
        &mut svm,
        &authority,
        &[&authority],
        &[ix_register_provider(
            &authority.pubkey(),
            provider_id,
            [7u8; 32],
            [8u8; 32],
        )],
    )
    .expect("register");
    dump("register", &m2.logs);

    let m3 = send(
        &mut svm,
        &authority,
        &[&authority],
        &[ix_dispatch_reputation_to_tempo(
            &authority.pubkey(),
            &Pubkey::new_unique(),
            provider_id,
            123_456_789,
            vec![0xde, 0xad, 0xbe, 0xef],
            vec![],
        )],
    )
    .expect("dispatch");
    dump("dispatch", &m3.logs);
}

/// Captures `Program data:` lines for every staking + burn-ladder event.
/// Walks the full lifecycle: init_staking -> set_stake_mint -> stake ->
/// request_unstake -> claim_unstake -> slash -> process_routing_fee_burn ->
/// reputation_vote_burn -> dispatch_stake_to_tempo. Each instruction emits
/// exactly one Anchor event so the dump is straightforward.
#[test]
fn dumps_staking_and_burn_log_lines() {
    let (mut svm, authority) = setup_svm();
    let provider_id = [42u8; 32];

    send(
        &mut svm,
        &authority,
        &[&authority],
        &[ix_initialize(&authority.pubkey(), authority.pubkey())],
    )
    .expect("initialize");

    let m_init = send(
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
    dump("init_staking", &m_init.logs);

    let mint_kp = create_mint(&mut svm, &authority, &authority.pubkey(), 6);
    let mint = mint_kp.pubkey();

    let m_set = send(
        &mut svm,
        &authority,
        &[&authority],
        &[ix_set_stake_mint(&authority.pubkey(), mint)],
    )
    .expect("set_stake_mint");
    dump("set_stake_mint", &m_set.logs);

    // Create a staker with a funded ATA.
    let staker = Keypair::new();
    svm.airdrop(&staker.pubkey(), 10_000_000_000)
        .expect("airdrop staker");
    let staker_ata =
        create_associated_token_account(&mut svm, &authority, &staker.pubkey(), &mint);
    let initial = 10 * MIN_STAKE_DEFAULT;
    mint_to(&mut svm, &authority, &mint, &staker_ata, &authority, initial);

    let stake_amount = 5 * MIN_STAKE_DEFAULT;
    let m_stake = send(
        &mut svm,
        &staker,
        &[&staker],
        &[ix_stake(&staker.pubkey(), &mint, provider_id, stake_amount)],
    )
    .expect("stake");
    dump("stake", &m_stake.logs);

    let unstake_amount = 2 * MIN_STAKE_DEFAULT;
    let m_req = send(
        &mut svm,
        &staker,
        &[&staker],
        &[ix_request_unstake(&staker.pubkey(), provider_id, unstake_amount)],
    )
    .expect("request_unstake");
    dump("request_unstake", &m_req.logs);

    warp_clock(&mut svm, COOLDOWN_DEFAULT_SECS + 1);
    svm.expire_blockhash();

    let m_claim = send(
        &mut svm,
        &staker,
        &[&staker],
        &[ix_claim_unstake(&staker.pubkey(), &mint, provider_id)],
    )
    .expect("claim_unstake");
    dump("claim_unstake", &m_claim.logs);

    // Slash requires the burn destination + its ATA to exist.
    ensure_system_account(&mut svm, &BURN_DESTINATION, 1_000_000_000);
    let _burn_ata =
        create_associated_token_account(&mut svm, &authority, &BURN_DESTINATION, &mint);

    let m_slash = send(
        &mut svm,
        &authority,
        &[&authority],
        &[ix_slash(
            &authority.pubkey(),
            &mint,
            provider_id,
            staker.pubkey(),
            MIN_STAKE_DEFAULT,
        )],
    )
    .expect("slash");
    dump("slash", &m_slash.logs);

    // Routing-fee burn pulls from the authority's own ATA.
    let authority_ata =
        create_associated_token_account(&mut svm, &authority, &authority.pubkey(), &mint);
    mint_to(&mut svm, &authority, &mint, &authority_ata, &authority, 10_000_000);
    let m_routing = send(
        &mut svm,
        &authority,
        &[&authority],
        &[ix_process_routing_fee_burn(&authority.pubkey(), &mint, 1_000_001)],
    )
    .expect("process_routing_fee_burn");
    dump("process_routing_fee_burn", &m_routing.logs);

    // Reputation-vote burn: voter is a fresh keypair with exactly one vote
    // worth of $GAZE.
    let voter = Keypair::new();
    svm.airdrop(&voter.pubkey(), 10_000_000_000)
        .expect("airdrop voter");
    let voter_ata =
        create_associated_token_account(&mut svm, &authority, &voter.pubkey(), &mint);
    mint_to(&mut svm, &authority, &mint, &voter_ata, &authority, VOTE_BURN_AMOUNT);
    let m_vote = send(
        &mut svm,
        &voter,
        &[&voter],
        &[ix_reputation_vote_burn(&voter.pubkey(), &mint, provider_id)],
    )
    .expect("reputation_vote_burn");
    dump("reputation_vote_burn", &m_vote.logs);

    // Dispatch the per-staker stake snapshot via CCIP.
    let m_dispatch_stake = send(
        &mut svm,
        &authority,
        &[&authority],
        &[ix_dispatch_stake_to_tempo(
            &authority.pubkey(),
            &Pubkey::new_unique(),
            provider_id,
            staker.pubkey(),
            123_456_789,
            vec![0xde, 0xad, 0xbe, 0xef],
            vec![],
        )],
    )
    .expect("dispatch_stake_to_tempo");
    dump("dispatch_stake_to_tempo", &m_dispatch_stake.logs);
}
