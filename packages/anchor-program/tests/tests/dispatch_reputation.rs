use solana_sdk::{
    instruction::Instruction,
    message::Message,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use stargaze_anchor::CcipDispatched;
use stargaze_anchor_tests::{
    find_event, ix_dispatch_reputation_to_tempo, ix_initialize, ix_register_provider, setup_svm,
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

const PROVIDER_ID: [u8; 32] = [4u8; 32];
const TEMPO_RECEIVER: [u8; 20] = [
    0xde, 0xad, 0xbe, 0xef, 0xfe, 0xed, 0xfa, 0xce, 0xca, 0xfe, 0xba, 0xbe, 0xc0, 0x01, 0xb0, 0x0b,
    0x10, 0x10, 0xab, 0xcd,
];
const DEST_SELECTOR: u64 = 16_015_286_601_757_825_753; // example: ethereum-sepolia CCIP selector

#[test]
fn dispatches_and_encodes_payload() {
    let (mut svm, authority) = setup_svm();
    let ccip_router = Pubkey::new_unique();

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
            PROVIDER_ID,
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
            PROVIDER_ID,
            DEST_SELECTOR,
            TEMPO_RECEIVER.to_vec(),
            vec![],
        )],
    )
    .expect("dispatch");

    let evt: CcipDispatched = find_event(&meta.logs).expect("CcipDispatched event present");
    assert_eq!(evt.provider_id, PROVIDER_ID);
    assert_eq!(evt.score, 500); // neutral midpoint from register_provider
    assert_eq!(evt.dest_chain_selector, DEST_SELECTOR);
    assert_eq!(evt.receiver, TEMPO_RECEIVER.to_vec());

    // ABI: bytes32 providerId || 30 zero bytes || uint16 score (big-endian).
    assert_eq!(evt.payload.len(), 64);
    assert_eq!(&evt.payload[..32], &PROVIDER_ID);
    assert!(evt.payload[32..62].iter().all(|b| *b == 0));
    assert_eq!(u16::from_be_bytes([evt.payload[62], evt.payload[63]]), 500);
}

#[test]
fn unauthorised_sender_rejected() {
    let (mut svm, authority) = setup_svm();
    let ccip_router = Pubkey::new_unique();

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
            PROVIDER_ID,
            [0u8; 32],
            [0u8; 32],
        )],
    )
    .expect("register");

    let attacker = Keypair::new();
    svm.airdrop(&attacker.pubkey(), 1_000_000_000)
        .expect("airdrop attacker");

    let err = send(
        &mut svm,
        &attacker,
        &[&attacker],
        &[ix_dispatch_reputation_to_tempo(
            &attacker.pubkey(),
            &ccip_router,
            PROVIDER_ID,
            DEST_SELECTOR,
            TEMPO_RECEIVER.to_vec(),
            vec![],
        )],
    )
    .expect_err("non-authority must be rejected");
    let _ = err;
}
