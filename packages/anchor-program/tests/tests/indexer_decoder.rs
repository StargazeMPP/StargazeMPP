//! End-to-end test that runs the `stargaze-events` decoder against the
//! actual `Program data: …` log lines emitted by `stargaze_anchor` under
//! litesvm. Pins decoder ↔ on-chain ABI compatibility: if either side
//! changes the event layout, the round-trip here breaks immediately
//! instead of silently corrupting indexer rows in production.
//!
//! Each test stands up a fresh litesvm context, invokes the instruction
//! that emits the event under test, then asserts
//! `stargaze_events::decode_logs(&meta.logs)` produces exactly one
//! decoded event of the expected variant with field-by-field equality
//! against the inputs.

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
    compute_signals_hash, create_associated_token_account, create_mint, ensure_system_account,
    ix_cast_reputation_vote, ix_ccip_mirror_score, ix_claim_unstake, ix_configure_vault,
    ix_deactivate_vault, ix_dispatch_reputation_to_tempo, ix_dispatch_stake_to_tempo,
    ix_init_escrow, ix_init_staking, ix_initialize, ix_process_routing_fee_burn,
    ix_record_x402_receipt, ix_register_provider, ix_reputation_vote_burn, ix_request_unstake,
    ix_set_reputation_score, ix_set_stake_mint, ix_set_vault_auditor_key,
    ix_set_vault_buyer_key_rotation_cid, ix_slash, ix_stake, ix_submit_vault_proof, mint_to,
    setup_svm, setup_svm_with_verifiers, warp_clock, BURN_DESTINATION,
};
use stargaze_events::{decode_logs, DecodedEvent, PubkeyBytes, VaultTier as EvVaultTier};

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

/// Pull the single decoded event of a given variant from the transaction
/// logs. Panics if zero or more than one matching event is present —
/// each instruction under test emits exactly one Anchor event.
fn decode_single(logs: &[String]) -> DecodedEvent {
    let events = decode_logs(logs);
    assert_eq!(
        events.len(),
        1,
        "expected exactly one decoded Anchor event in tx logs, got {}: {:?}",
        events.len(),
        events.iter().map(|e| e.name()).collect::<Vec<_>>(),
    );
    events.into_iter().next().unwrap()
}

/// Bring the staking system up to a usable state with a real mint and seeded
/// staker balance. Returns `(mint_kp, staker, staker_ata)`. Crib of the
/// `bootstrap_with_mint` helper from `staking.rs` — kept local because it's
/// only test-specific bootstrap glue.
fn bootstrap_with_mint(
    svm: &mut litesvm::LiteSVM,
    authority: &Keypair,
    initial_balance: u64,
) -> (Keypair, Keypair, Pubkey) {
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

    let staker = Keypair::new();
    svm.airdrop(&staker.pubkey(), 10_000_000_000)
        .expect("airdrop staker");
    let staker_ata = create_associated_token_account(svm, authority, &staker.pubkey(), &mint);
    mint_to(svm, authority, &mint, &staker_ata, authority, initial_balance);

    (mint_kp, staker, staker_ata)
}

#[test]
fn decodes_provider_registered_from_litesvm() {
    let (mut svm, authority) = setup_svm();
    let provider_id = [1u8; 32];
    let category_hash = [2u8; 32];
    let meta_cid = [3u8; 32];

    send(
        &mut svm,
        &authority,
        &[&authority],
        &[ix_initialize(&authority.pubkey(), authority.pubkey())],
    )
    .expect("initialize");

    let meta = send(
        &mut svm,
        &authority,
        &[&authority],
        &[ix_register_provider(
            &authority.pubkey(),
            provider_id,
            category_hash,
            meta_cid,
        )],
    )
    .expect("register_provider");

    let DecodedEvent::ProviderRegistered(e) = decode_single(&meta.logs) else {
        panic!("expected ProviderRegistered");
    };
    assert_eq!(e.provider_id, provider_id);
    assert_eq!(e.owner, PubkeyBytes(authority.pubkey().to_bytes()));
    assert_eq!(e.category_hash, category_hash);
    assert_eq!(e.meta_cid, meta_cid);
}

#[test]
fn decodes_reputation_voted_from_litesvm() {
    let (mut svm, authority) = setup_svm();
    let provider_id = [9u8; 32];

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
        &[ix_register_provider(
            &authority.pubkey(),
            provider_id,
            [0u8; 32],
            [0u8; 32],
        )],
    )
    .expect("register");

    let voter = Keypair::new();
    svm.airdrop(&voter.pubkey(), 1_000_000_000)
        .expect("airdrop voter");

    let meta = send(
        &mut svm,
        &voter,
        &[&voter],
        &[ix_cast_reputation_vote(&voter.pubkey(), provider_id, true)],
    )
    .expect("vote");

    let DecodedEvent::ReputationVoted(e) = decode_single(&meta.logs) else {
        panic!("expected ReputationVoted");
    };
    assert_eq!(e.provider_id, provider_id);
    assert_eq!(e.voter, PubkeyBytes(voter.pubkey().to_bytes()));
    assert!(e.accurate);
}

#[test]
fn decodes_x402_receipt_recorded_from_litesvm() {
    let (mut svm, payer) = setup_svm();
    let session_id = [5u8; 32];
    let provider_id = [6u8; 32];
    let amount: u64 = 1_234_567;

    send(
        &mut svm,
        &payer,
        &[&payer],
        &[ix_initialize(&payer.pubkey(), payer.pubkey())],
    )
    .expect("initialize");

    let meta = send(
        &mut svm,
        &payer,
        &[&payer],
        &[ix_record_x402_receipt(
            &payer.pubkey(),
            session_id,
            provider_id,
            amount,
        )],
    )
    .expect("record_x402_receipt");

    let DecodedEvent::X402ReceiptRecorded(e) = decode_single(&meta.logs) else {
        panic!("expected X402ReceiptRecorded");
    };
    assert_eq!(e.session_id, session_id);
    assert_eq!(e.provider_id, provider_id);
    assert_eq!(e.payer, PubkeyBytes(payer.pubkey().to_bytes()));
    assert_eq!(e.amount, amount);
    // setup_svm pins unix_timestamp to 1_700_000_000.
    assert_eq!(e.paid_at, 1_700_000_000);
}

#[test]
fn decodes_reputation_mirrored_from_litesvm() {
    let (mut svm, authority) = setup_svm();
    let provider_id = [11u8; 32];

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
        &[ix_register_provider(
            &authority.pubkey(),
            provider_id,
            [0u8; 32],
            [0u8; 32],
        )],
    )
    .expect("register");

    let meta = send(
        &mut svm,
        &authority,
        &[&authority],
        &[ix_ccip_mirror_score(&authority.pubkey(), provider_id, 875)],
    )
    .expect("ccip_mirror_score");

    let DecodedEvent::ReputationMirrored(e) = decode_single(&meta.logs) else {
        panic!("expected ReputationMirrored");
    };
    assert_eq!(e.provider_id, provider_id);
    assert_eq!(e.score, 875);
}

#[test]
fn decodes_ccip_dispatched_from_litesvm() {
    let (mut svm, authority) = setup_svm();
    let ccip_router = Pubkey::new_unique();
    let provider_id = [12u8; 32];
    let dest_selector: u64 = 16_015_286_601_757_825_753;
    let receiver = vec![0xde, 0xad, 0xbe, 0xef];

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
        &[ix_register_provider(
            &authority.pubkey(),
            provider_id,
            [0u8; 32],
            [0u8; 32],
        )],
    )
    .expect("register");

    let meta = send(
        &mut svm,
        &authority,
        &[&authority],
        &[ix_dispatch_reputation_to_tempo(
            &authority.pubkey(),
            &ccip_router,
            provider_id,
            dest_selector,
            receiver.clone(),
            vec![],
        )],
    )
    .expect("dispatch");

    let DecodedEvent::CcipDispatched(e) = decode_single(&meta.logs) else {
        panic!("expected CcipDispatched");
    };
    assert_eq!(e.provider_id, provider_id);
    assert_eq!(e.score, 500); // neutral midpoint set by register_provider
    assert_eq!(e.dest_chain_selector, dest_selector);
    assert_eq!(e.receiver, receiver);
    // ABI: bytes32 providerId || 30 zero bytes || uint16 score (big-endian).
    assert_eq!(e.payload.len(), 64);
    assert_eq!(&e.payload[..32], &provider_id);
    assert!(e.payload[32..62].iter().all(|b| *b == 0));
    assert_eq!(u16::from_be_bytes([e.payload[62], e.payload[63]]), 500);
    assert!(e.extra_args.is_empty());
}

#[test]
fn decodes_staking_initialized_from_litesvm() {
    let (mut svm, authority) = setup_svm();

    send(
        &mut svm,
        &authority,
        &[&authority],
        &[ix_initialize(&authority.pubkey(), authority.pubkey())],
    )
    .expect("initialize");

    let meta = send(
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

    let DecodedEvent::StakingInitialized(e) = decode_single(&meta.logs) else {
        panic!("expected StakingInitialized");
    };
    assert_eq!(e.stake_mint, PubkeyBytes(Pubkey::default().to_bytes()));
    assert_eq!(e.min_stake, MIN_STAKE_DEFAULT);
    assert_eq!(e.verified_stake, VERIFIED_STAKE_DEFAULT);
    assert_eq!(e.cooldown_secs, COOLDOWN_DEFAULT_SECS);
}

#[test]
fn decodes_stake_mint_set_from_litesvm() {
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

    let mint_kp = create_mint(&mut svm, &authority, &authority.pubkey(), 6);
    let meta = send(
        &mut svm,
        &authority,
        &[&authority],
        &[ix_set_stake_mint(&authority.pubkey(), mint_kp.pubkey())],
    )
    .expect("set_stake_mint");

    let DecodedEvent::StakeMintSet(e) = decode_single(&meta.logs) else {
        panic!("expected StakeMintSet");
    };
    assert_eq!(e.stake_mint, PubkeyBytes(mint_kp.pubkey().to_bytes()));
}

#[test]
fn decodes_staked_from_litesvm() {
    let (mut svm, authority) = setup_svm();
    let provider_id = [13u8; 32];
    let initial = 5 * MIN_STAKE_DEFAULT;
    let (mint_kp, staker, _staker_ata) = bootstrap_with_mint(&mut svm, &authority, initial);
    let mint = mint_kp.pubkey();

    let stake_amount = 2 * MIN_STAKE_DEFAULT;
    let meta = send(
        &mut svm,
        &staker,
        &[&staker],
        &[ix_stake(&staker.pubkey(), &mint, provider_id, stake_amount)],
    )
    .expect("stake");

    let DecodedEvent::Staked(e) = decode_single(&meta.logs) else {
        panic!("expected Staked");
    };
    assert_eq!(e.provider_id, provider_id);
    assert_eq!(e.owner, PubkeyBytes(staker.pubkey().to_bytes()));
    assert_eq!(e.amount, stake_amount);
    assert_eq!(e.total, stake_amount);
}

#[test]
fn decodes_unstake_requested_from_litesvm() {
    let (mut svm, authority) = setup_svm();
    let provider_id = [14u8; 32];
    let initial = 5 * MIN_STAKE_DEFAULT;
    let (mint_kp, staker, _staker_ata) = bootstrap_with_mint(&mut svm, &authority, initial);
    let mint = mint_kp.pubkey();

    let stake_amount = 3 * MIN_STAKE_DEFAULT;
    send(
        &mut svm,
        &staker,
        &[&staker],
        &[ix_stake(&staker.pubkey(), &mint, provider_id, stake_amount)],
    )
    .expect("stake");

    let unstake_amount = 2 * MIN_STAKE_DEFAULT;
    let meta = send(
        &mut svm,
        &staker,
        &[&staker],
        &[ix_request_unstake(&staker.pubkey(), provider_id, unstake_amount)],
    )
    .expect("request_unstake");

    let DecodedEvent::UnstakeRequested(e) = decode_single(&meta.logs) else {
        panic!("expected UnstakeRequested");
    };
    assert_eq!(e.provider_id, provider_id);
    assert_eq!(e.owner, PubkeyBytes(staker.pubkey().to_bytes()));
    assert_eq!(e.amount, unstake_amount);
    // setup_svm pins unix_timestamp to 1_700_000_000.
    assert_eq!(e.cooldown_until, 1_700_000_000 + COOLDOWN_DEFAULT_SECS);
}

#[test]
fn decodes_unstaked_from_litesvm() {
    let (mut svm, authority) = setup_svm();
    let provider_id = [15u8; 32];
    let initial = 5 * MIN_STAKE_DEFAULT;
    let (mint_kp, staker, _staker_ata) = bootstrap_with_mint(&mut svm, &authority, initial);
    let mint = mint_kp.pubkey();

    let stake_amount = 3 * MIN_STAKE_DEFAULT;
    send(
        &mut svm,
        &staker,
        &[&staker],
        &[ix_stake(&staker.pubkey(), &mint, provider_id, stake_amount)],
    )
    .expect("stake");

    let unstake_amount = 2 * MIN_STAKE_DEFAULT;
    send(
        &mut svm,
        &staker,
        &[&staker],
        &[ix_request_unstake(&staker.pubkey(), provider_id, unstake_amount)],
    )
    .expect("request_unstake");

    warp_clock(&mut svm, COOLDOWN_DEFAULT_SECS + 1);
    svm.expire_blockhash();

    let meta = send(
        &mut svm,
        &staker,
        &[&staker],
        &[ix_claim_unstake(&staker.pubkey(), &mint, provider_id)],
    )
    .expect("claim_unstake");

    let DecodedEvent::Unstaked(e) = decode_single(&meta.logs) else {
        panic!("expected Unstaked");
    };
    assert_eq!(e.provider_id, provider_id);
    assert_eq!(e.owner, PubkeyBytes(staker.pubkey().to_bytes()));
    assert_eq!(e.amount, unstake_amount);
}

#[test]
fn decodes_slashed_from_litesvm() {
    let (mut svm, authority) = setup_svm();
    let provider_id = [16u8; 32];
    let initial = 6 * MIN_STAKE_DEFAULT;
    let (mint_kp, staker, _staker_ata) = bootstrap_with_mint(&mut svm, &authority, initial);
    let mint = mint_kp.pubkey();

    ensure_system_account(&mut svm, &BURN_DESTINATION, 1_000_000_000);
    let _burn_ata =
        create_associated_token_account(&mut svm, &authority, &BURN_DESTINATION, &mint);

    let stake_amount = 4 * MIN_STAKE_DEFAULT;
    send(
        &mut svm,
        &staker,
        &[&staker],
        &[ix_stake(&staker.pubkey(), &mint, provider_id, stake_amount)],
    )
    .expect("stake");

    let slash_amount = MIN_STAKE_DEFAULT;
    let meta = send(
        &mut svm,
        &authority,
        &[&authority],
        &[ix_slash(
            &authority.pubkey(),
            &mint,
            provider_id,
            staker.pubkey(),
            slash_amount,
        )],
    )
    .expect("slash");

    let DecodedEvent::Slashed(e) = decode_single(&meta.logs) else {
        panic!("expected Slashed");
    };
    assert_eq!(e.provider_id, provider_id);
    assert_eq!(e.owner, PubkeyBytes(staker.pubkey().to_bytes()));
    assert_eq!(e.amount, slash_amount);
    assert_eq!(e.destination, PubkeyBytes(BURN_DESTINATION.to_bytes()));
}

#[test]
fn decodes_routing_fee_processed_from_litesvm() {
    let (mut svm, authority) = setup_svm();

    // Bootstrap with authority-held balance (the routing-fee burn pulls from
    // the authority's ATA). We reuse `bootstrap_with_mint` and then mint into
    // the authority's ATA separately.
    let (mint_kp, _staker, _staker_ata) = bootstrap_with_mint(&mut svm, &authority, 0);
    let mint = mint_kp.pubkey();

    let authority_ata =
        create_associated_token_account(&mut svm, &authority, &authority.pubkey(), &mint);
    let initial: u64 = 10_000_000;
    mint_to(&mut svm, &authority, &mint, &authority_ata, &authority, initial);

    let amount: u64 = 1_000_001; // odd amount so the extra unit routes to stakers
    let meta = send(
        &mut svm,
        &authority,
        &[&authority],
        &[ix_process_routing_fee_burn(&authority.pubkey(), &mint, amount)],
    )
    .expect("process_routing_fee_burn");

    let DecodedEvent::RoutingFeeProcessed(e) = decode_single(&meta.logs) else {
        panic!("expected RoutingFeeProcessed");
    };
    let burned = amount / 2;
    let to_stakers = amount - burned;
    assert_eq!(e.burned, burned);
    assert_eq!(e.to_stakers, to_stakers);
}

#[test]
fn decodes_reputation_vote_burned_from_litesvm() {
    let (mut svm, authority) = setup_svm();
    let provider_id = [17u8; 32];

    let (mint_kp, _staker, _staker_ata) = bootstrap_with_mint(&mut svm, &authority, 0);
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
        &[ix_reputation_vote_burn(&voter.pubkey(), &mint, provider_id)],
    )
    .expect("reputation_vote_burn");

    let DecodedEvent::ReputationVoteBurned(e) = decode_single(&meta.logs) else {
        panic!("expected ReputationVoteBurned");
    };
    assert_eq!(e.voter, PubkeyBytes(voter.pubkey().to_bytes()));
    assert_eq!(e.provider_id, provider_id);
}

#[test]
fn decodes_stake_dispatched_from_litesvm() {
    let (mut svm, authority) = setup_svm();
    let provider_id = [18u8; 32];
    let ccip_router = Pubkey::new_unique();
    let dest_selector: u64 = 16_015_286_601_757_825_753;
    let receiver = vec![0xde, 0xad, 0xbe, 0xef];

    let initial = 5 * MIN_STAKE_DEFAULT;
    let (mint_kp, staker, _staker_ata) = bootstrap_with_mint(&mut svm, &authority, initial);
    let mint = mint_kp.pubkey();

    let stake_amount: u64 = 100_000_000;
    send(
        &mut svm,
        &staker,
        &[&staker],
        &[ix_stake(&staker.pubkey(), &mint, provider_id, stake_amount)],
    )
    .expect("stake");

    let meta = send(
        &mut svm,
        &authority,
        &[&authority],
        &[ix_dispatch_stake_to_tempo(
            &authority.pubkey(),
            &ccip_router,
            provider_id,
            staker.pubkey(),
            dest_selector,
            receiver.clone(),
            vec![],
        )],
    )
    .expect("dispatch_stake_to_tempo");

    let DecodedEvent::StakeDispatched(e) = decode_single(&meta.logs) else {
        panic!("expected StakeDispatched");
    };
    assert_eq!(e.provider_id, provider_id);
    assert_eq!(e.owner, PubkeyBytes(staker.pubkey().to_bytes()));
    assert_eq!(e.amount, stake_amount);
    assert_eq!(e.dest_chain_selector, dest_selector);
    assert_eq!(e.receiver, receiver);
    // ABI: bytes32 providerId || 12 zero pad || owner[12..32] || 24 zero pad || u64 BE.
    assert_eq!(e.payload.len(), 96);
    assert_eq!(&e.payload[0..32], &provider_id);
    assert!(e.payload[32..44].iter().all(|b| *b == 0));
    assert_eq!(&e.payload[44..64], &staker.pubkey().to_bytes()[12..32]);
    assert!(e.payload[64..88].iter().all(|b| *b == 0));
    assert_eq!(
        u64::from_be_bytes(e.payload[88..96].try_into().unwrap()),
        stake_amount,
    );
    assert!(e.extra_args.is_empty());
}

#[test]
fn decodes_reputation_score_set_from_litesvm() {
    let (mut svm, authority) = setup_svm();
    let provider_id = [19u8; 32];

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
        &[ix_register_provider(
            &authority.pubkey(),
            provider_id,
            [0u8; 32],
            [0u8; 32],
        )],
    )
    .expect("register");

    let meta = send(
        &mut svm,
        &authority,
        &[&authority],
        &[ix_set_reputation_score(&authority.pubkey(), provider_id, 720)],
    )
    .expect("set_reputation_score");

    let DecodedEvent::ReputationScoreSet(e) = decode_single(&meta.logs) else {
        panic!("expected ReputationScoreSet");
    };
    assert_eq!(e.provider_id, provider_id);
    assert_eq!(e.score, 720);
}

#[test]
fn decodes_escrow_initialized_from_litesvm() {
    let (mut svm, authority) = setup_svm();
    let usdc_mint = Pubkey::new_unique();
    let router = Pubkey::new_unique();

    send(
        &mut svm,
        &authority,
        &[&authority],
        &[ix_initialize(&authority.pubkey(), authority.pubkey())],
    )
    .expect("initialize");

    let meta = send(
        &mut svm,
        &authority,
        &[&authority],
        &[ix_init_escrow(&authority.pubkey(), usdc_mint, router)],
    )
    .expect("init_escrow");

    let DecodedEvent::EscrowInitialized(e) = decode_single(&meta.logs) else {
        panic!("expected EscrowInitialized");
    };
    assert_eq!(e.admin, PubkeyBytes(authority.pubkey().to_bytes()));
    assert_eq!(e.usdc_mint, PubkeyBytes(usdc_mint.to_bytes()));
    assert_eq!(e.router, PubkeyBytes(router.to_bytes()));
}

#[test]
fn decodes_vault_configured_from_litesvm() {
    let (mut svm, authority) = setup_svm();
    let provider_id = [20u8; 32];
    let verifier = Pubkey::new_unique();
    let arweave_cid = [0x42u8; 32];

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
        &[ix_register_provider(
            &authority.pubkey(),
            provider_id,
            [0u8; 32],
            [0u8; 32],
        )],
    )
    .expect("register");

    let meta = send(
        &mut svm,
        &authority,
        &[&authority],
        &[ix_configure_vault(
            &authority.pubkey(),
            provider_id,
            stargaze_anchor::VaultTier::ZkAggregate,
            verifier,
            arweave_cid,
        )],
    )
    .expect("configure_vault");

    let DecodedEvent::VaultConfigured(e) = decode_single(&meta.logs) else {
        panic!("expected VaultConfigured");
    };
    assert_eq!(e.provider_id, provider_id);
    assert_eq!(e.tier, EvVaultTier::ZkAggregate);
    assert_eq!(e.on_chain_verifier, PubkeyBytes(verifier.to_bytes()));
    assert_eq!(e.arweave_cid, arweave_cid);
}

#[test]
fn decodes_vault_auditor_key_set_from_litesvm() {
    let (mut svm, authority) = setup_svm();
    let provider_id = [21u8; 32];
    let verifier = Pubkey::new_unique();
    let auditor = Pubkey::new_unique();

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
        &[ix_register_provider(
            &authority.pubkey(),
            provider_id,
            [0u8; 32],
            [0u8; 32],
        )],
    )
    .expect("register");
    send(
        &mut svm,
        &authority,
        &[&authority],
        &[ix_configure_vault(
            &authority.pubkey(),
            provider_id,
            stargaze_anchor::VaultTier::Confidential,
            verifier,
            [0u8; 32],
        )],
    )
    .expect("configure_vault");

    let meta = send(
        &mut svm,
        &authority,
        &[&authority],
        &[ix_set_vault_auditor_key(&authority.pubkey(), provider_id, auditor)],
    )
    .expect("set_vault_auditor_key");

    let DecodedEvent::VaultAuditorKeySet(e) = decode_single(&meta.logs) else {
        panic!("expected VaultAuditorKeySet");
    };
    assert_eq!(e.provider_id, provider_id);
    // First call from default — previous is `Pubkey::default()`.
    assert_eq!(e.previous, PubkeyBytes([0u8; 32]));
    assert_eq!(e.current, PubkeyBytes(auditor.to_bytes()));
}

#[test]
fn decodes_vault_buyer_key_rotation_updated_from_litesvm() {
    let (mut svm, authority) = setup_svm();
    let provider_id = [22u8; 32];
    let verifier = Pubkey::new_unique();
    let cid = [0x77u8; 32];

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
        &[ix_register_provider(
            &authority.pubkey(),
            provider_id,
            [0u8; 32],
            [0u8; 32],
        )],
    )
    .expect("register");
    send(
        &mut svm,
        &authority,
        &[&authority],
        &[ix_configure_vault(
            &authority.pubkey(),
            provider_id,
            stargaze_anchor::VaultTier::BuyerKey,
            verifier,
            [0u8; 32],
        )],
    )
    .expect("configure_vault");

    let meta = send(
        &mut svm,
        &authority,
        &[&authority],
        &[ix_set_vault_buyer_key_rotation_cid(
            &authority.pubkey(),
            provider_id,
            cid,
        )],
    )
    .expect("set_vault_buyer_key_rotation_cid");

    let DecodedEvent::VaultBuyerKeyRotationUpdated(e) = decode_single(&meta.logs) else {
        panic!("expected VaultBuyerKeyRotationUpdated");
    };
    assert_eq!(e.provider_id, provider_id);
    assert_eq!(e.cid, cid);
}

#[test]
fn decodes_vault_deactivated_from_litesvm() {
    let (mut svm, authority) = setup_svm();
    let provider_id = [23u8; 32];
    let verifier = Pubkey::new_unique();

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
        &[ix_register_provider(
            &authority.pubkey(),
            provider_id,
            [0u8; 32],
            [0u8; 32],
        )],
    )
    .expect("register");
    send(
        &mut svm,
        &authority,
        &[&authority],
        &[ix_configure_vault(
            &authority.pubkey(),
            provider_id,
            stargaze_anchor::VaultTier::ZkAggregate,
            verifier,
            [0u8; 32],
        )],
    )
    .expect("configure_vault");

    let meta = send(
        &mut svm,
        &authority,
        &[&authority],
        &[ix_deactivate_vault(&authority.pubkey(), provider_id)],
    )
    .expect("deactivate_vault");

    let DecodedEvent::VaultDeactivated(e) = decode_single(&meta.logs) else {
        panic!("expected VaultDeactivated");
    };
    assert_eq!(e.provider_id, provider_id);
}

#[test]
fn decodes_vault_proof_verified_from_litesvm() {
    // Reuse the aggregate_sum fixture from submit_vault_proof.rs so we don't
    // duplicate the proof bytes here. Keeping the integration footprint to a
    // single circuit is enough to prove the cross-crate decoder pipes the
    // VaultProofVerified shape end-to-end.
    const AGGREGATE_SUM_PROOF: [u8; 256] = [
        15, 94, 189, 196, 250, 135, 63, 120, 66, 154, 206, 209, 207, 9, 103, 101,
        33, 52, 161, 131, 252, 55, 15, 118, 88, 245, 200, 32, 195, 190, 50, 150,
        16, 210, 197, 242, 114, 3, 184, 136, 148, 205, 10, 202, 112, 56, 91, 220,
        243, 32, 153, 101, 37, 72, 181, 220, 94, 71, 181, 69, 234, 25, 2, 75,
        23, 88, 81, 147, 128, 139, 98, 215, 168, 53, 164, 223, 105, 51, 119, 60,
        148, 153, 49, 135, 193, 144, 176, 68, 227, 129, 119, 109, 239, 9, 214, 58,
        8, 206, 79, 71, 212, 66, 102, 55, 176, 95, 142, 53, 231, 210, 227, 86,
        182, 174, 138, 114, 19, 162, 25, 229, 34, 120, 172, 29, 15, 120, 89, 181,
        14, 116, 9, 165, 156, 223, 182, 168, 208, 209, 182, 128, 221, 85, 245, 160,
        112, 135, 42, 253, 51, 109, 72, 225, 106, 92, 82, 119, 8, 229, 167, 169,
        38, 167, 201, 96, 220, 218, 132, 107, 217, 218, 52, 245, 224, 243, 16, 30,
        71, 70, 157, 189, 252, 148, 183, 165, 36, 170, 170, 119, 134, 114, 24, 221,
        19, 61, 50, 61, 93, 233, 79, 200, 101, 181, 148, 113, 38, 135, 216, 230,
        57, 19, 196, 158, 1, 140, 10, 134, 162, 177, 163, 175, 129, 92, 138, 172,
        19, 9, 229, 10, 169, 241, 227, 102, 36, 48, 138, 36, 162, 187, 149, 60,
        125, 69, 184, 248, 174, 238, 9, 107, 74, 193, 1, 57, 79, 198, 156, 136,
    ];
    const AGGREGATE_SUM_PUBLIC: [u8; 32] = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        36,
    ];

    let (mut svm, authority) = setup_svm_with_verifiers();
    let provider_id = [24u8; 32];

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
        &[ix_register_provider(
            &authority.pubkey(),
            provider_id,
            [0u8; 32],
            [0u8; 32],
        )],
    )
    .expect("register");
    send(
        &mut svm,
        &authority,
        &[&authority],
        &[ix_configure_vault(
            &authority.pubkey(),
            provider_id,
            stargaze_anchor::VaultTier::ZkAggregate,
            vault_verifier_aggregate_sum::ID,
            [0u8; 32],
        )],
    )
    .expect("configure_vault");

    let signals = vec![AGGREGATE_SUM_PUBLIC];
    let signals_hash = compute_signals_hash(&signals);

    let meta = send(
        &mut svm,
        &authority,
        &[&authority],
        &[ix_submit_vault_proof(
            &authority.pubkey(),
            vault_verifier_aggregate_sum::ID,
            provider_id,
            signals_hash,
            AGGREGATE_SUM_PROOF,
            signals,
        )],
    )
    .expect("submit_vault_proof");

    let DecodedEvent::VaultProofVerified(e) = decode_single(&meta.logs) else {
        panic!("expected VaultProofVerified");
    };
    assert_eq!(e.provider_id, provider_id);
    assert_eq!(e.tier, EvVaultTier::ZkAggregate);
    assert_eq!(e.signals_hash, signals_hash);
    assert_eq!(e.submitter, PubkeyBytes(authority.pubkey().to_bytes()));
}
