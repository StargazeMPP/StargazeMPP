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
    build_ed25519_ix, build_voucher_message, compute_signals_hash,
    create_associated_token_account, create_mint, ensure_system_account, ix_claim_unstake,
    ix_close_session, ix_configure_vault, ix_deactivate_vault, ix_init_escrow, ix_init_staking,
    ix_initialize, ix_open_session, ix_process_routing_fee_burn, ix_register_provider,
    ix_reputation_vote_burn, ix_request_unstake, ix_set_reputation_score, ix_set_stake_mint,
    ix_set_vault_auditor_key, ix_set_vault_buyer_key_rotation_cid, ix_settle, ix_slash, ix_stake,
    ix_submit_vault_proof, mint_to, setup_svm, setup_svm_with_verifiers, sign_voucher,
    voucher_message_hash, warp_clock, BURN_DESTINATION,
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

/// Captures `Program data:` lines for every staking + burn-ladder event.
/// Walks the full lifecycle: init_staking -> set_stake_mint -> stake ->
/// request_unstake -> claim_unstake -> slash -> process_routing_fee_burn ->
/// reputation_vote_burn. Each instruction emits exactly one Anchor event so
/// the dump is straightforward.
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
}

/// Captures `Program data:` lines for the escrow + vault registry + verifier
/// events introduced by the Solana-only pivot. Walks: init_escrow ->
/// set_reputation_score -> configure_vault -> set_vault_auditor_key ->
/// set_vault_buyer_key_rotation_cid -> deactivate_vault ->
/// submit_vault_proof. Each step emits exactly one Anchor event.
#[test]
fn dumps_vault_and_verifier_log_lines() {
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
    let provider_id = [99u8; 32];

    send(
        &mut svm,
        &authority,
        &[&authority],
        &[ix_initialize(&authority.pubkey(), authority.pubkey())],
    )
    .expect("initialize");

    let m_escrow = send(
        &mut svm,
        &authority,
        &[&authority],
        &[ix_init_escrow(
            &authority.pubkey(),
            Pubkey::new_unique(),
            Pubkey::new_unique(),
        )],
    )
    .expect("init_escrow");
    dump("init_escrow", &m_escrow.logs);

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
    .expect("register_provider");

    let m_score = send(
        &mut svm,
        &authority,
        &[&authority],
        &[ix_set_reputation_score(&authority.pubkey(), provider_id, 720)],
    )
    .expect("set_reputation_score");
    dump("set_reputation_score", &m_score.logs);

    let m_cfg = send(
        &mut svm,
        &authority,
        &[&authority],
        &[ix_configure_vault(
            &authority.pubkey(),
            provider_id,
            stargaze_anchor::VaultTier::ZkAggregate,
            vault_verifier_aggregate_sum::ID,
            [0x44u8; 32],
        )],
    )
    .expect("configure_vault");
    dump("configure_vault", &m_cfg.logs);

    let m_auditor = send(
        &mut svm,
        &authority,
        &[&authority],
        &[ix_set_vault_auditor_key(
            &authority.pubkey(),
            provider_id,
            Pubkey::new_unique(),
        )],
    )
    .expect("set_vault_auditor_key");
    dump("set_vault_auditor_key", &m_auditor.logs);

    let m_rot = send(
        &mut svm,
        &authority,
        &[&authority],
        &[ix_set_vault_buyer_key_rotation_cid(
            &authority.pubkey(),
            provider_id,
            [0x55u8; 32],
        )],
    )
    .expect("set_vault_buyer_key_rotation_cid");
    dump("set_vault_buyer_key_rotation_cid", &m_rot.logs);

    // Submit the proof before deactivating — VaultProofVerified requires an
    // active vault. (Deactivation is the last step in this dump.)
    let signals = vec![AGGREGATE_SUM_PUBLIC];
    let signals_hash = compute_signals_hash(&signals);
    let m_proof = send(
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
    dump("submit_vault_proof", &m_proof.logs);

    let m_off = send(
        &mut svm,
        &authority,
        &[&authority],
        &[ix_deactivate_vault(&authority.pubkey(), provider_id)],
    )
    .expect("deactivate_vault");
    dump("deactivate_vault", &m_off.logs);
}

/// Captures `Program data:` lines for the full escrow voucher-settle
/// lifecycle: init_escrow -> open_session -> settle (ed25519 precompile +
/// program ix in the same tx) -> close_session. Each step emits exactly one
/// Anchor `emit!` event; the settle tx also carries the Ed25519 precompile
/// instruction (which does not emit a log line) before the program ix. This
/// gives the indexer crate a real-tape fixture for SessionOpened,
/// VoucherSettled, and SessionSettled in one place.
#[test]
fn dumps_escrow_log_lines() {
    let (mut svm, authority) = setup_svm();
    let deposit: u64 = 10_000_000;

    send(
        &mut svm,
        &authority,
        &[&authority],
        &[ix_initialize(&authority.pubkey(), authority.pubkey())],
    )
    .expect("initialize");

    let router = Keypair::new();
    svm.airdrop(&router.pubkey(), 10_000_000_000).expect("airdrop router");

    let mint_kp = create_mint(&mut svm, &authority, &authority.pubkey(), 6);
    let mint = mint_kp.pubkey();

    let m_init = send(
        &mut svm,
        &authority,
        &[&authority],
        &[ix_init_escrow(&authority.pubkey(), mint, router.pubkey())],
    )
    .expect("init_escrow");
    dump("init_escrow", &m_init.logs);

    let agent = Keypair::new();
    svm.airdrop(&agent.pubkey(), 10_000_000_000).expect("airdrop agent");
    let agent_ata =
        create_associated_token_account(&mut svm, &authority, &agent.pubkey(), &mint);
    mint_to(&mut svm, &authority, &mint, &agent_ata, &authority, deposit);

    let provider_owner = Keypair::new();
    svm.airdrop(&provider_owner.pubkey(), 10_000_000_000)
        .expect("airdrop provider_owner");
    let _provider_ata = create_associated_token_account(
        &mut svm,
        &authority,
        &provider_owner.pubkey(),
        &mint,
    );

    let session_id = [0x33u8; 32];
    let provider_id = [0x34u8; 32];
    let expires_at = 1_700_000_000 + 3_600;

    let m_open = send(
        &mut svm,
        &agent,
        &[&agent],
        &[ix_open_session(
            &agent.pubkey(),
            &mint,
            session_id,
            deposit,
            deposit,
            expires_at,
        )],
    )
    .expect("open_session");
    dump("open_session", &m_open.logs);

    let cumulative: u64 = deposit / 2;
    let nonce: u64 = 1;
    let message = build_voucher_message(
        &session_id,
        &agent.pubkey(),
        &provider_id,
        cumulative,
        nonce,
    );
    let signature = sign_voucher(&agent, &message);
    let hash = voucher_message_hash(&message);
    let ed_ix = build_ed25519_ix(&agent.pubkey(), &signature, &message);
    let settle_ix = ix_settle(
        &router.pubkey(),
        session_id,
        provider_id,
        &provider_owner.pubkey(),
        &mint,
        cumulative,
        nonce,
        hash,
    );
    let m_settle = send(&mut svm, &router, &[&router], &[ed_ix, settle_ix])
        .expect("settle");
    dump("settle", &m_settle.logs);

    let m_close = send(
        &mut svm,
        &router,
        &[&router],
        &[ix_close_session(
            &router.pubkey(),
            session_id,
            &agent.pubkey(),
            &mint,
        )],
    )
    .expect("close_session");
    dump("close_session", &m_close.logs);
}
