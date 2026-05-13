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
use stargaze_anchor_tests::{
    ix_dispatch_reputation_to_tempo, ix_initialize, ix_register_provider, setup_svm,
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
