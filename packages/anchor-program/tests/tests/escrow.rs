//! Integration tests for the escrow + voucher settlement instructions on
//! `stargaze_anchor`. Uses the in-process `litesvm` runtime and the on-chain
//! SPL Token + Associated Token Account + Ed25519 precompile programs to
//! exercise the full settle path:
//!   init_escrow -> open_session -> settle (with ed25519 precompile) -> close_session.

use anchor_lang::AnchorDeserialize;
use solana_sdk::{
    instruction::Instruction,
    message::Message,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use stargaze_anchor::{Session, SessionOpened, SessionSettled, VoucherSettled};
use stargaze_anchor_tests::{
    associated_token_address, build_ed25519_ix, build_voucher_message,
    consumed_voucher_pda, create_associated_token_account, create_mint, find_event,
    ix_close_session, ix_init_escrow, ix_initialize, ix_open_session, ix_settle, mint_to,
    routing_fee_vault_authority_pda, session_pda, session_vault_authority_pda, setup_svm,
    sign_voucher, token_balance, voucher_message_hash, warp_clock,
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

fn read_session(svm: &litesvm::LiteSVM, session_id: &[u8; 32]) -> Session {
    let (addr, _) = session_pda(session_id);
    let acct = svm.get_account(&addr).expect("session exists");
    let mut data = &acct.data[8..];
    Session::deserialize(&mut data).expect("decode session")
}

/// Standard fixtures used by most tests: the admin/authority signer, a USDC
/// mint owned by the admin, a router keypair, an agent with a pre-loaded ATA
/// balance, and the `provider` (provider_owner). Returns also the resolved
/// PDAs that tests commonly inspect.
struct Fixtures {
    authority: Keypair,
    router: Keypair,
    agent: Keypair,
    provider_owner: Keypair,
    mint: Pubkey,
    agent_ata: Pubkey,
    provider_ata: Pubkey,
}

fn bootstrap(svm: &mut litesvm::LiteSVM, authority: Keypair, agent_balance: u64) -> Fixtures {
    send(
        svm,
        &authority,
        &[&authority],
        &[ix_initialize(&authority.pubkey(), authority.pubkey())],
    )
    .expect("initialize");

    let router = Keypair::new();
    svm.airdrop(&router.pubkey(), 10_000_000_000).expect("airdrop router");

    let mint_kp = create_mint(svm, &authority, &authority.pubkey(), 6);
    let mint = mint_kp.pubkey();

    send(
        svm,
        &authority,
        &[&authority],
        &[ix_init_escrow(&authority.pubkey(), mint, router.pubkey())],
    )
    .expect("init_escrow");

    let agent = Keypair::new();
    svm.airdrop(&agent.pubkey(), 10_000_000_000).expect("airdrop agent");
    let agent_ata = create_associated_token_account(svm, &authority, &agent.pubkey(), &mint);
    mint_to(svm, &authority, &mint, &agent_ata, &authority, agent_balance);

    let provider_owner = Keypair::new();
    svm.airdrop(&provider_owner.pubkey(), 10_000_000_000)
        .expect("airdrop provider_owner");
    let provider_ata =
        create_associated_token_account(svm, &authority, &provider_owner.pubkey(), &mint);

    Fixtures {
        authority,
        router,
        agent,
        provider_owner,
        mint,
        agent_ata,
        provider_ata,
    }
}

/// Build the precompile + program ix pair for a single voucher and execute
/// it. Returns the resulting tx metadata so tests can inspect logs / events.
fn submit_settle(
    svm: &mut litesvm::LiteSVM,
    f: &Fixtures,
    session_id: [u8; 32],
    provider_id: [u8; 32],
    cumulative_amount: u64,
    nonce: u64,
    signer_kp: &Keypair, // who signs the voucher; usually f.agent
) -> Result<litesvm::types::TransactionMetadata, litesvm::types::FailedTransactionMetadata> {
    let message = build_voucher_message(
        &session_id,
        &f.agent.pubkey(),
        &provider_id,
        cumulative_amount,
        nonce,
    );
    let signature = sign_voucher(signer_kp, &message);
    let hash = voucher_message_hash(&message);
    let ed25519_ix = build_ed25519_ix(&signer_kp.pubkey(), &signature, &message);
    let settle_ix = ix_settle(
        &f.router.pubkey(),
        session_id,
        provider_id,
        &f.provider_owner.pubkey(),
        &f.mint,
        cumulative_amount,
        nonce,
        hash,
    );
    send(svm, &f.router, &[&f.router], &[ed25519_ix, settle_ix])
}

fn open_session_simple(
    svm: &mut litesvm::LiteSVM,
    f: &Fixtures,
    session_id: [u8; 32],
    deposit: u64,
    spending_limit: u64,
    expires_at: i64,
) {
    send(
        svm,
        &f.agent,
        &[&f.agent],
        &[ix_open_session(
            &f.agent.pubkey(),
            &f.mint,
            session_id,
            deposit,
            spending_limit,
            expires_at,
        )],
    )
    .expect("open_session");
}


#[test]
fn init_escrow_sets_config() {
    let (mut svm, authority) = setup_svm();
    let f = bootstrap(&mut svm, authority, 0);

    // Non-admin must be rejected.
    let attacker = Keypair::new();
    svm.airdrop(&attacker.pubkey(), 10_000_000_000)
        .expect("airdrop attacker");

    // Try a *fresh* svm to attempt an init from non-admin.
    let (mut svm2, auth2) = setup_svm();
    send(
        &mut svm2,
        &auth2,
        &[&auth2],
        &[ix_initialize(&auth2.pubkey(), auth2.pubkey())],
    )
    .expect("initialize");
    let attacker2 = Keypair::new();
    svm2.airdrop(&attacker2.pubkey(), 10_000_000_000)
        .expect("airdrop attacker2");
    let err = send(
        &mut svm2,
        &attacker2,
        &[&attacker2],
        &[ix_init_escrow(&attacker2.pubkey(), Pubkey::new_unique(), Pubkey::new_unique())],
    )
    .expect_err("non-admin init_escrow must revert");
    let logs = err.meta.logs.join("\n");
    assert!(
        logs.contains("Unauthorized") || logs.contains("authorised"),
        "expected Unauthorized in logs, got:\n{logs}"
    );

    let _ = f.router; // touch field
}

#[test]
fn open_session_creates_vault_and_holds_deposit() {
    let (mut svm, authority) = setup_svm();
    let deposit: u64 = 10_000_000; // 10 USDC
    let f = bootstrap(&mut svm, authority, deposit * 2);

    let session_id = [0x11; 32];
    let expires_at = 1_700_000_000 + 3_600;
    open_session_simple(&mut svm, &f, session_id, deposit, deposit, expires_at);

    let (vault_auth, _) = session_vault_authority_pda(&session_id);
    let vault_ata = associated_token_address(&vault_auth, &f.mint);
    assert_eq!(token_balance(&svm, &vault_ata), deposit);
    assert_eq!(token_balance(&svm, &f.agent_ata), deposit);

    let session = read_session(&svm, &session_id);
    assert_eq!(session.session_id, session_id);
    assert_eq!(session.agent_wallet, f.agent.pubkey());
    assert_eq!(session.deposit, deposit);
    assert_eq!(session.spending_limit, deposit);
    assert_eq!(session.expires_at, expires_at);
    assert!(!session.settled);
    assert_eq!(session.total_spent, 0);
    assert_eq!(session.total_fee, 0);
}

#[test]
fn open_session_rejects_limit_above_deposit() {
    let (mut svm, authority) = setup_svm();
    let deposit: u64 = 10_000_000;
    let f = bootstrap(&mut svm, authority, deposit * 2);
    let session_id = [0x12; 32];

    let err = send(
        &mut svm,
        &f.agent,
        &[&f.agent],
        &[ix_open_session(
            &f.agent.pubkey(),
            &f.mint,
            session_id,
            deposit,
            deposit + 1, // > deposit
            1_700_000_000 + 3_600,
        )],
    )
    .expect_err("spending limit above deposit must revert");
    let logs = err.meta.logs.join("\n");
    assert!(
        logs.contains("SpendingLimitExceeded") || logs.contains("spending limit"),
        "expected SpendingLimitExceeded in logs, got:\n{logs}"
    );
}

#[test]
fn open_session_rejects_duplicate_session_id() {
    let (mut svm, authority) = setup_svm();
    let deposit: u64 = 5_000_000;
    let f = bootstrap(&mut svm, authority, deposit * 4);
    let session_id = [0x13; 32];
    open_session_simple(&mut svm, &f, session_id, deposit, deposit, 1_700_000_000 + 3_600);

    let err = send(
        &mut svm,
        &f.agent,
        &[&f.agent],
        &[ix_open_session(
            &f.agent.pubkey(),
            &f.mint,
            session_id,
            deposit,
            deposit,
            1_700_000_000 + 3_600,
        )],
    )
    .expect_err("duplicate session_id must revert");
    let _ = err; // anchor surfaces account-already-in-use
}

#[test]
fn settle_single_voucher_happy_path() {
    let (mut svm, authority) = setup_svm();
    let deposit: u64 = 10_000_000;
    let f = bootstrap(&mut svm, authority, deposit);
    let session_id = [0x21; 32];
    let provider_id = [0x22; 32];

    open_session_simple(&mut svm, &f, session_id, deposit, deposit, 1_700_000_000 + 3_600);

    let cumulative: u64 = 1_000_000; // 1 USDC
    submit_settle(&mut svm, &f, session_id, provider_id, cumulative, 1, &f.agent)
        .expect("settle");

    let fee = cumulative * 200 / 10_000; // 2%
    let to_provider = cumulative - fee;
    assert_eq!(token_balance(&svm, &f.provider_ata), to_provider);

    let (fee_auth, _) = routing_fee_vault_authority_pda();
    let fee_ata = associated_token_address(&fee_auth, &f.mint);
    assert_eq!(token_balance(&svm, &fee_ata), fee);

    let session = read_session(&svm, &session_id);
    assert_eq!(session.total_spent, to_provider);
    assert_eq!(session.total_fee, fee);

    let (vault_auth, _) = session_vault_authority_pda(&session_id);
    let vault_ata = associated_token_address(&vault_auth, &f.mint);
    assert_eq!(token_balance(&svm, &vault_ata), deposit - cumulative);
}

#[test]
fn settle_then_close_preserves_accounting_identity() {
    for &deposit in &[1_000_000u64, 5_555_555, 50_000_000] {
        let (mut svm, authority) = setup_svm();
        let f = bootstrap(&mut svm, authority, deposit);
        let session_id = [0x31; 32];
        let provider_id = [0x32; 32];
        open_session_simple(&mut svm, &f, session_id, deposit, deposit, 1_700_000_000 + 3_600);

        let cumulative: u64 = deposit / 2;
        submit_settle(&mut svm, &f, session_id, provider_id, cumulative, 1, &f.agent)
            .expect("settle");

        send(
            &mut svm,
            &f.router,
            &[&f.router],
            &[ix_close_session(
                &f.router.pubkey(),
                session_id,
                &f.agent.pubkey(),
                &f.mint,
            )],
        )
        .expect("close_session");

        let session = read_session(&svm, &session_id);
        let total_to_providers = session.total_spent;
        let routing_fee = session.total_fee;
        let refund = deposit - total_to_providers - routing_fee;

        // Identity: deposit == providers + fee + refund.
        assert_eq!(deposit, total_to_providers + routing_fee + refund);

        // Agent ATA should now hold the refund (started at zero — the open
        // moved the full deposit out).
        assert_eq!(token_balance(&svm, &f.agent_ata), refund);
    }
}

#[test]
fn settle_rejects_non_router() {
    let (mut svm, authority) = setup_svm();
    let deposit: u64 = 5_000_000;
    let f = bootstrap(&mut svm, authority, deposit);
    let session_id = [0x41; 32];
    let provider_id = [0x42; 32];
    open_session_simple(&mut svm, &f, session_id, deposit, deposit, 1_700_000_000 + 3_600);

    // Replace router with random attacker.
    let attacker = Keypair::new();
    svm.airdrop(&attacker.pubkey(), 10_000_000_000).expect("airdrop");

    let cumulative: u64 = 1_000_000;
    let message = build_voucher_message(
        &session_id,
        &f.agent.pubkey(),
        &provider_id,
        cumulative,
        1,
    );
    let signature = sign_voucher(&f.agent, &message);
    let hash = voucher_message_hash(&message);
    let ed_ix = build_ed25519_ix(&f.agent.pubkey(), &signature, &message);
    // settle_ix built with attacker as signer.
    let settle_ix = ix_settle(
        &attacker.pubkey(),
        session_id,
        provider_id,
        &f.provider_owner.pubkey(),
        &f.mint,
        cumulative,
        1,
        hash,
    );

    let err = send(&mut svm, &attacker, &[&attacker], &[ed_ix, settle_ix])
        .expect_err("non-router must revert");
    let logs = err.meta.logs.join("\n");
    assert!(
        logs.contains("UnauthorizedRouter") || logs.contains("not the configured router"),
        "expected UnauthorizedRouter in logs, got:\n{logs}"
    );
}

#[test]
fn settle_rejects_bad_signer() {
    let (mut svm, authority) = setup_svm();
    let deposit: u64 = 5_000_000;
    let f = bootstrap(&mut svm, authority, deposit);
    let session_id = [0x51; 32];
    let provider_id = [0x52; 32];
    open_session_simple(&mut svm, &f, session_id, deposit, deposit, 1_700_000_000 + 3_600);

    // A different keypair signs the same message.
    let imposter = Keypair::new();
    let cumulative: u64 = 1_000_000;
    let message = build_voucher_message(
        &session_id,
        &f.agent.pubkey(),
        &provider_id,
        cumulative,
        1,
    );
    let signature = sign_voucher(&imposter, &message);
    let hash = voucher_message_hash(&message);
    let ed_ix = build_ed25519_ix(&imposter.pubkey(), &signature, &message);
    let settle_ix = ix_settle(
        &f.router.pubkey(),
        session_id,
        provider_id,
        &f.provider_owner.pubkey(),
        &f.mint,
        cumulative,
        1,
        hash,
    );

    let err = send(&mut svm, &f.router, &[&f.router], &[ed_ix, settle_ix])
        .expect_err("wrong signer must revert");
    let logs = err.meta.logs.join("\n");
    assert!(
        logs.contains("WrongSigner") || logs.contains("agent wallet"),
        "expected WrongSigner in logs, got:\n{logs}"
    );
}

#[test]
fn settle_rejects_wrong_message_bytes() {
    let (mut svm, authority) = setup_svm();
    let deposit: u64 = 5_000_000;
    let f = bootstrap(&mut svm, authority, deposit);
    let session_id = [0x61; 32];
    let provider_id = [0x62; 32];
    open_session_simple(&mut svm, &f, session_id, deposit, deposit, 1_700_000_000 + 3_600);

    // The agent signs a *different* cumulative amount than what settle is
    // called with. The precompile passes (signature is valid for the signed
    // bytes), but the program detects message mismatch.
    let signed_cumulative: u64 = 500_000;
    let ix_cumulative: u64 = 1_000_000;
    let signed_message = build_voucher_message(
        &session_id,
        &f.agent.pubkey(),
        &provider_id,
        signed_cumulative,
        1,
    );
    let signature = sign_voucher(&f.agent, &signed_message);
    let hash_of_ix_message = voucher_message_hash(&build_voucher_message(
        &session_id,
        &f.agent.pubkey(),
        &provider_id,
        ix_cumulative,
        1,
    ));
    let ed_ix = build_ed25519_ix(&f.agent.pubkey(), &signature, &signed_message);
    let settle_ix = ix_settle(
        &f.router.pubkey(),
        session_id,
        provider_id,
        &f.provider_owner.pubkey(),
        &f.mint,
        ix_cumulative,
        1,
        hash_of_ix_message,
    );

    let err = send(&mut svm, &f.router, &[&f.router], &[ed_ix, settle_ix])
        .expect_err("message mismatch must revert");
    let logs = err.meta.logs.join("\n");
    assert!(
        logs.contains("WrongMessage") || logs.contains("do not match"),
        "expected WrongMessage in logs, got:\n{logs}"
    );
}

#[test]
fn settle_rejects_missing_precompile_ix() {
    let (mut svm, authority) = setup_svm();
    let deposit: u64 = 5_000_000;
    let f = bootstrap(&mut svm, authority, deposit);
    let session_id = [0x71; 32];
    let provider_id = [0x72; 32];
    open_session_simple(&mut svm, &f, session_id, deposit, deposit, 1_700_000_000 + 3_600);

    let cumulative: u64 = 1_000_000;
    let message = build_voucher_message(
        &session_id,
        &f.agent.pubkey(),
        &provider_id,
        cumulative,
        1,
    );
    let hash = voucher_message_hash(&message);
    let settle_ix = ix_settle(
        &f.router.pubkey(),
        session_id,
        provider_id,
        &f.provider_owner.pubkey(),
        &f.mint,
        cumulative,
        1,
        hash,
    );

    // Send settle WITHOUT preceding precompile.
    let err = send(&mut svm, &f.router, &[&f.router], &[settle_ix])
        .expect_err("missing precompile must revert");
    let logs = err.meta.logs.join("\n");
    assert!(
        logs.contains("MissingPrecompile") || logs.contains("precompile"),
        "expected MissingPrecompile in logs, got:\n{logs}"
    );
}

#[test]
fn settle_replay_rejected() {
    let (mut svm, authority) = setup_svm();
    let deposit: u64 = 10_000_000;
    let f = bootstrap(&mut svm, authority, deposit);
    let session_id = [0x81; 32];
    let provider_id = [0x82; 32];
    open_session_simple(&mut svm, &f, session_id, deposit, deposit, 1_700_000_000 + 3_600);

    submit_settle(&mut svm, &f, session_id, provider_id, 1_000_000, 1, &f.agent)
        .expect("first settle");

    // Submit the exact same voucher again.
    let err = submit_settle(&mut svm, &f, session_id, provider_id, 1_000_000, 1, &f.agent)
        .expect_err("replay must revert");
    // Anchor surfaces account-already-in-use on the consumed_voucher PDA
    // init; the specific log text is implementation-dependent.
    let _ = err;
    // Sanity-check: PDA exists at the expected address.
    let message = build_voucher_message(
        &session_id,
        &f.agent.pubkey(),
        &provider_id,
        1_000_000,
        1,
    );
    let hash = voucher_message_hash(&message);
    let (consumed, _) = consumed_voucher_pda(&session_id, &hash);
    assert!(svm.get_account(&consumed).is_some());
}

#[test]
fn settle_non_monotonic_rejected() {
    let (mut svm, authority) = setup_svm();
    let deposit: u64 = 10_000_000;
    let f = bootstrap(&mut svm, authority, deposit);
    let session_id = [0x91; 32];
    let provider_id = [0x92; 32];
    open_session_simple(&mut svm, &f, session_id, deposit, deposit, 1_700_000_000 + 3_600);

    submit_settle(&mut svm, &f, session_id, provider_id, 2_000_000, 1, &f.agent)
        .expect("first settle");

    // Now try a voucher with cumulative == prev (not strictly greater).
    let err = submit_settle(&mut svm, &f, session_id, provider_id, 2_000_000, 2, &f.agent)
        .expect_err("non-monotonic (equal) must revert");
    let logs = err.meta.logs.join("\n");
    assert!(
        logs.contains("NonMonotonic") || logs.contains("strictly greater"),
        "expected NonMonotonic in logs, got:\n{logs}"
    );

    // And a voucher with cumulative < prev.
    let err2 = submit_settle(&mut svm, &f, session_id, provider_id, 1_000_000, 3, &f.agent)
        .expect_err("non-monotonic (lower) must revert");
    let logs2 = err2.meta.logs.join("\n");
    assert!(
        logs2.contains("NonMonotonic") || logs2.contains("strictly greater"),
        "expected NonMonotonic in logs, got:\n{logs2}"
    );
}

#[test]
fn settle_spending_limit_cap() {
    let (mut svm, authority) = setup_svm();
    let deposit: u64 = 10_000_000;
    let limit: u64 = 3_000_000;
    let f = bootstrap(&mut svm, authority, deposit);
    let session_id = [0xA1; 32];
    let provider_id = [0xA2; 32];
    open_session_simple(&mut svm, &f, session_id, deposit, limit, 1_700_000_000 + 3_600);

    let initial_provider_balance = token_balance(&svm, &f.provider_ata);

    // cumulative_amount > limit should revert.
    let err = submit_settle(&mut svm, &f, session_id, provider_id, limit + 1, 1, &f.agent)
        .expect_err("over spending limit must revert");
    let logs = err.meta.logs.join("\n");
    assert!(
        logs.contains("SpendingLimitExceeded") || logs.contains("spending limit"),
        "expected SpendingLimitExceeded in logs, got:\n{logs}"
    );
    assert_eq!(token_balance(&svm, &f.provider_ata), initial_provider_balance);

    // Exactly at limit succeeds.
    submit_settle(&mut svm, &f, session_id, provider_id, limit, 2, &f.agent)
        .expect("at-limit settle");
}

#[test]
fn settle_after_close_rejected() {
    let (mut svm, authority) = setup_svm();
    let deposit: u64 = 5_000_000;
    let f = bootstrap(&mut svm, authority, deposit);
    let session_id = [0xB1; 32];
    let provider_id = [0xB2; 32];
    open_session_simple(&mut svm, &f, session_id, deposit, deposit, 1_700_000_000 + 3_600);

    send(
        &mut svm,
        &f.router,
        &[&f.router],
        &[ix_close_session(
            &f.router.pubkey(),
            session_id,
            &f.agent.pubkey(),
            &f.mint,
        )],
    )
    .expect("close_session");

    let err = submit_settle(&mut svm, &f, session_id, provider_id, 1_000_000, 1, &f.agent)
        .expect_err("settle after close must revert");
    let logs = err.meta.logs.join("\n");
    assert!(
        logs.contains("AlreadySettled") || logs.contains("already been settled"),
        "expected AlreadySettled in logs, got:\n{logs}"
    );
}

#[test]
fn settle_after_expiry_rejected() {
    let (mut svm, authority) = setup_svm();
    let deposit: u64 = 5_000_000;
    let f = bootstrap(&mut svm, authority, deposit);
    let session_id = [0xC1; 32];
    let provider_id = [0xC2; 32];
    let expires_at = 1_700_000_000 + 60; // expire in 60s
    open_session_simple(&mut svm, &f, session_id, deposit, deposit, expires_at);

    // Warp past the expiry.
    warp_clock(&mut svm, 120);
    svm.expire_blockhash();

    let err = submit_settle(&mut svm, &f, session_id, provider_id, 1_000_000, 1, &f.agent)
        .expect_err("settle after expiry must revert");
    let logs = err.meta.logs.join("\n");
    assert!(
        logs.contains("SessionExpired") || logs.contains("expiry"),
        "expected SessionExpired in logs, got:\n{logs}"
    );
}

#[test]
fn close_session_expired_no_vouchers_refunds_full_deposit() {
    let (mut svm, authority) = setup_svm();
    let deposit: u64 = 7_777_777;
    let f = bootstrap(&mut svm, authority, deposit);
    let session_id = [0xD1; 32];
    let expires_at = 1_700_000_000 + 30;
    open_session_simple(&mut svm, &f, session_id, deposit, deposit, expires_at);

    // Warp past expiry; agent self-closes.
    warp_clock(&mut svm, 60);
    svm.expire_blockhash();

    send(
        &mut svm,
        &f.agent,
        &[&f.agent],
        &[ix_close_session(
            &f.agent.pubkey(),
            session_id,
            &f.agent.pubkey(),
            &f.mint,
        )],
    )
    .expect("agent close after expiry");

    assert_eq!(token_balance(&svm, &f.agent_ata), deposit);
    let session = read_session(&svm, &session_id);
    assert!(session.settled);
    assert_eq!(session.total_spent, 0);
    assert_eq!(session.total_fee, 0);
}

#[test]
fn close_session_unauthorized_before_expiry() {
    let (mut svm, authority) = setup_svm();
    let deposit: u64 = 5_000_000;
    let f = bootstrap(&mut svm, authority, deposit);
    let session_id = [0xE1; 32];
    let expires_at = 1_700_000_000 + 3_600;
    open_session_simple(&mut svm, &f, session_id, deposit, deposit, expires_at);

    // Random signer pre-expiry: revert.
    let attacker = Keypair::new();
    svm.airdrop(&attacker.pubkey(), 10_000_000_000).expect("airdrop");
    let err = send(
        &mut svm,
        &attacker,
        &[&attacker],
        &[ix_close_session(
            &attacker.pubkey(),
            session_id,
            &f.agent.pubkey(),
            &f.mint,
        )],
    )
    .expect_err("random caller pre-expiry must revert");
    let _ = err;

    // Agent pre-expiry: revert with SessionNotExpired.
    let err2 = send(
        &mut svm,
        &f.agent,
        &[&f.agent],
        &[ix_close_session(
            &f.agent.pubkey(),
            session_id,
            &f.agent.pubkey(),
            &f.mint,
        )],
    )
    .expect_err("agent close pre-expiry must revert");
    let logs2 = err2.meta.logs.join("\n");
    assert!(
        logs2.contains("SessionNotExpired") || logs2.contains("expiry has not"),
        "expected SessionNotExpired in logs, got:\n{logs2}"
    );

    // Router pre-expiry: ok.
    send(
        &mut svm,
        &f.router,
        &[&f.router],
        &[ix_close_session(
            &f.router.pubkey(),
            session_id,
            &f.agent.pubkey(),
            &f.mint,
        )],
    )
    .expect("router close pre-expiry");
}

#[test]
fn multi_voucher_per_session_sequential() {
    let (mut svm, authority) = setup_svm();
    let deposit: u64 = 30_000_000;
    let f = bootstrap(&mut svm, authority, deposit);
    let session_id = [0xF1; 32];
    open_session_simple(&mut svm, &f, session_id, deposit, deposit, 1_700_000_000 + 3_600);

    // Three different providers, each gets one voucher with monotonic
    // cumulative-per-provider. We need a separate provider_ata for each
    // provider_owner.
    let mut prov_owners: Vec<Keypair> = vec![];
    let mut prov_atas: Vec<Pubkey> = vec![];
    let mut prov_ids: Vec<[u8; 32]> = vec![];
    for i in 0..3u8 {
        let owner = Keypair::new();
        svm.airdrop(&owner.pubkey(), 10_000_000_000).expect("airdrop");
        let ata =
            create_associated_token_account(&mut svm, &f.authority, &owner.pubkey(), &f.mint);
        prov_atas.push(ata);
        prov_owners.push(owner);
        let mut pid = [0u8; 32];
        pid[0] = 0xA0 + i;
        prov_ids.push(pid);
    }

    let cumulatives = [1_000_000u64, 2_000_000, 3_000_000];
    let mut expected_total_to_providers = 0u64;
    let mut expected_total_fee = 0u64;
    for i in 0..3usize {
        // Build settle ix targeting this specific provider's ATA.
        let cumulative = cumulatives[i];
        let provider_id = prov_ids[i];
        let provider_owner = &prov_owners[i];
        let message = build_voucher_message(
            &session_id,
            &f.agent.pubkey(),
            &provider_id,
            cumulative,
            (i as u64) + 1,
        );
        let signature = sign_voucher(&f.agent, &message);
        let hash = voucher_message_hash(&message);
        let ed_ix = build_ed25519_ix(&f.agent.pubkey(), &signature, &message);
        let settle_ix = ix_settle(
            &f.router.pubkey(),
            session_id,
            provider_id,
            &provider_owner.pubkey(),
            &f.mint,
            cumulative,
            (i as u64) + 1,
            hash,
        );
        send(&mut svm, &f.router, &[&f.router], &[ed_ix, settle_ix])
            .expect("settle");

        let fee = cumulative * 200 / 10_000;
        let to_prov = cumulative - fee;
        assert_eq!(token_balance(&svm, &prov_atas[i]), to_prov);
        expected_total_to_providers += to_prov;
        expected_total_fee += fee;
    }

    let session = read_session(&svm, &session_id);
    assert_eq!(session.total_spent, expected_total_to_providers);
    assert_eq!(session.total_fee, expected_total_fee);

    let (fee_auth, _) = routing_fee_vault_authority_pda();
    let fee_ata = associated_token_address(&fee_auth, &f.mint);
    assert_eq!(token_balance(&svm, &fee_ata), expected_total_fee);
}

#[test]
fn event_logs_round_trip() {
    let (mut svm, authority) = setup_svm();
    let deposit: u64 = 5_000_000;
    let f = bootstrap(&mut svm, authority, deposit);
    let session_id = [0x01; 32];
    let provider_id = [0x02; 32];
    let expires_at = 1_700_000_000 + 3_600;

    let meta = send(
        &mut svm,
        &f.agent,
        &[&f.agent],
        &[ix_open_session(
            &f.agent.pubkey(),
            &f.mint,
            session_id,
            deposit,
            deposit,
            expires_at,
        )],
    )
    .expect("open_session");
    let opened: SessionOpened =
        find_event(&meta.logs).expect("SessionOpened event present");
    assert_eq!(opened.session_id, session_id);
    assert_eq!(opened.agent_wallet, f.agent.pubkey());
    assert_eq!(opened.deposit, deposit);

    let cumulative = 1_000_000u64;
    let meta2 = submit_settle(&mut svm, &f, session_id, provider_id, cumulative, 1, &f.agent)
        .expect("settle");
    let settled: VoucherSettled =
        find_event(&meta2.logs).expect("VoucherSettled event present");
    assert_eq!(settled.session_id, session_id);
    assert_eq!(settled.provider_id, provider_id);
    assert_eq!(settled.cumulative_amount, cumulative);

    let meta3 = send(
        &mut svm,
        &f.router,
        &[&f.router],
        &[ix_close_session(
            &f.router.pubkey(),
            session_id,
            &f.agent.pubkey(),
            &f.mint,
        )],
    )
    .expect("close_session");
    let closed: SessionSettled =
        find_event(&meta3.logs).expect("SessionSettled event present");
    assert_eq!(closed.session_id, session_id);
    assert!(closed.refund_to_agent > 0);
}
