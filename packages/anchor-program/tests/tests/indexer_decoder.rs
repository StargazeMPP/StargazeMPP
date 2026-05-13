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
use stargaze_anchor_tests::{
    ix_cast_reputation_vote, ix_ccip_mirror_score, ix_dispatch_reputation_to_tempo,
    ix_initialize, ix_record_x402_receipt, ix_register_provider, setup_svm,
};
use stargaze_events::{decode_logs, DecodedEvent, PubkeyBytes};

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
