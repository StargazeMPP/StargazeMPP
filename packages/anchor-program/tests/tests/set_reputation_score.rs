use anchor_lang::AnchorDeserialize;
use solana_sdk::{
    instruction::Instruction,
    message::Message,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use stargaze_anchor::{Provider, ReputationScoreSet};
use stargaze_anchor_tests::{
    find_event, ix_initialize, ix_register_provider, ix_set_reputation_score, provider_pda,
    setup_svm,
};

const PROVIDER_ID: [u8; 32] = [22u8; 32];

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
fn set_reputation_score_happy_path() {
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
        &[ix_set_reputation_score(&authority.pubkey(), PROVIDER_ID, 875)],
    )
    .expect("set score");

    let evt: ReputationScoreSet = find_event(&meta.logs).expect("ReputationScoreSet event");
    assert_eq!(evt.provider_id, PROVIDER_ID);
    assert_eq!(evt.score, 875);

    let (provider_addr, _) = provider_pda(&PROVIDER_ID);
    let acct = svm.get_account(&provider_addr).expect("provider exists");
    let mut data = &acct.data[8..];
    let provider = Provider::deserialize(&mut data).expect("decode provider");
    assert_eq!(provider.reputation_score, 875);
}

#[test]
fn set_reputation_score_rejects_non_authority() {
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
        .expect("airdrop");

    let err = send(
        &mut svm,
        &attacker,
        &[&attacker],
        &[ix_set_reputation_score(&attacker.pubkey(), PROVIDER_ID, 700)],
    )
    .expect_err("non-authority must be rejected");
    let _ = err;
}

#[test]
fn set_reputation_score_rejects_out_of_range() {
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
        &[ix_register_provider(
            &authority.pubkey(),
            PROVIDER_ID,
            [0u8; 32],
            [0u8; 32],
        )],
    )
    .expect("register");

    let err = send(
        &mut svm,
        &authority,
        &[&authority],
        &[ix_set_reputation_score(&authority.pubkey(), PROVIDER_ID, 1001)],
    )
    .expect_err("score > 1000 must be rejected");
    let _ = err;
}
