//! Compute-unit benchmark for `settle`. Opens a fresh session per scenario
//! `n ∈ {1, 10, 50, 100, 200}`, sends `n` monotonically-increasing vouchers
//! against a single provider, and records `meta.compute_units_consumed` for
//! each settle tx. Prints a summary (`first`, `last`, `min`, `max`, `avg`)
//! per scenario.
//!
//! Each `settle` instruction is constant-time on-chain — the consumed-voucher
//! PDA is `init`-only and there's no iteration over prior vouchers — so the
//! numbers should be flat across `n`. The bench exists to **confirm** that:
//! if a future change introduces an O(n) scaling, this bench will surface
//! it and the off-chain Payment Router can adjust its batching budget.
//!
//! Run via `cargo test -p stargaze_anchor_tests --test escrow_bench --
//! --nocapture` to see the table.
//!
//! Numbers are recorded in `packages/anchor-program/BENCH.md`.

use solana_sdk::{
    instruction::Instruction,
    message::Message,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use stargaze_anchor_tests::{
    build_ed25519_ix, build_voucher_message, create_associated_token_account, create_mint,
    ix_init_escrow, ix_initialize, ix_open_session, ix_settle, mint_to, setup_svm, sign_voucher,
    voucher_message_hash,
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

struct Fixtures {
    router: Keypair,
    agent: Keypair,
    provider_owner: Keypair,
    mint: Pubkey,
}

fn bootstrap(svm: &mut litesvm::LiteSVM, authority: &Keypair, agent_balance: u64) -> Fixtures {
    send(
        svm,
        authority,
        &[authority],
        &[ix_initialize(&authority.pubkey(), authority.pubkey())],
    )
    .expect("initialize");

    let router = Keypair::new();
    svm.airdrop(&router.pubkey(), 10_000_000_000).expect("airdrop router");

    let mint_kp = create_mint(svm, authority, &authority.pubkey(), 6);
    let mint = mint_kp.pubkey();

    send(
        svm,
        authority,
        &[authority],
        &[ix_init_escrow(&authority.pubkey(), mint, router.pubkey())],
    )
    .expect("init_escrow");

    let agent = Keypair::new();
    svm.airdrop(&agent.pubkey(), 10_000_000_000).expect("airdrop agent");
    let agent_ata = create_associated_token_account(svm, authority, &agent.pubkey(), &mint);
    mint_to(svm, authority, &mint, &agent_ata, authority, agent_balance);

    let provider_owner = Keypair::new();
    svm.airdrop(&provider_owner.pubkey(), 10_000_000_000)
        .expect("airdrop provider_owner");
    let _provider_ata =
        create_associated_token_account(svm, authority, &provider_owner.pubkey(), &mint);

    Fixtures { router, agent, provider_owner, mint }
}

fn measure_run(svm: &mut litesvm::LiteSVM, f: &Fixtures, n: usize) -> Vec<u64> {
    let mut session_id = [0u8; 32];
    session_id[0] = n as u8;
    session_id[1] = 0xBE;
    let provider_id = [0xEF; 32];

    let per_voucher: u64 = 1_000; // 0.001 USDC
    let deposit: u64 = per_voucher * (n as u64) + 10_000_000;

    send(
        svm,
        &f.agent,
        &[&f.agent],
        &[ix_open_session(
            &f.agent.pubkey(),
            &f.mint,
            session_id,
            deposit,
            deposit,
            1_700_000_000 + 86_400,
        )],
    )
    .expect("open_session");

    let mut cus = Vec::with_capacity(n);
    for i in 0..n {
        let nonce = (i as u64) + 1;
        let cumulative = per_voucher * nonce;
        let message = build_voucher_message(
            &session_id,
            &f.agent.pubkey(),
            &provider_id,
            cumulative,
            nonce,
        );
        let signature = sign_voucher(&f.agent, &message);
        let hash = voucher_message_hash(&message);
        let ed_ix = build_ed25519_ix(&f.agent.pubkey(), &signature, &message);
        let settle_ix = ix_settle(
            &f.router.pubkey(),
            session_id,
            provider_id,
            &f.provider_owner.pubkey(),
            &f.mint,
            cumulative,
            nonce,
            hash,
        );
        let meta = send(svm, &f.router, &[&f.router], &[ed_ix, settle_ix])
            .unwrap_or_else(|e| panic!("settle {nonce} failed: {:?}", e.err));
        cus.push(meta.compute_units_consumed);
    }
    cus
}

#[test]
fn bench_settle_cu() {
    let (mut svm, authority) = setup_svm();
    // 1 USDC deposit ceiling per scenario; we open one session per `n` and
    // tear it down implicitly between runs by using a fresh session_id seed.
    let f = bootstrap(&mut svm, &authority, 1_000_000_000);

    println!();
    println!(
        "{:>5}  {:>10}  {:>10}  {:>10}  {:>10}  {:>10}",
        "n", "first", "last", "min", "max", "avg",
    );
    for &n in &[1usize, 10, 50, 100, 200] {
        let cus = measure_run(&mut svm, &f, n);
        let first = cus.first().copied().unwrap_or(0);
        let last = cus.last().copied().unwrap_or(0);
        let min = cus.iter().copied().min().unwrap_or(0);
        let max = cus.iter().copied().max().unwrap_or(0);
        let sum: u64 = cus.iter().sum();
        let avg = sum / (cus.len() as u64);
        println!(
            "{:>5}  {:>10}  {:>10}  {:>10}  {:>10}  {:>10}",
            n, first, last, min, max, avg,
        );
    }
}
