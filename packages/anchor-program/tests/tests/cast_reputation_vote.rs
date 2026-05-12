use solana_sdk::{
    instruction::Instruction,
    message::Message,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use stargaze_anchor::ReputationVoted;
use stargaze_anchor_tests::{
    find_event, ix_cast_reputation_vote, ix_initialize, ix_register_provider, setup_svm,
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

#[test]
fn emits_event() {
    let (mut svm, payer) = setup_svm();
    let provider_id: [u8; 32] = [9u8; 32];

    send(
        &mut svm,
        &payer,
        &[&payer],
        &[ix_initialize(&payer.pubkey(), payer.pubkey())],
    )
    .expect("initialize");
    send(
        &mut svm,
        &payer,
        &[&payer],
        &[ix_register_provider(
            &payer.pubkey(),
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

    let evt: ReputationVoted = find_event(&meta.logs).expect("ReputationVoted event present");
    assert_eq!(evt.provider_id, provider_id);
    assert_eq!(evt.voter, voter.pubkey());
    assert!(evt.accurate);
}
