use anchor_lang::AnchorDeserialize;
use solana_sdk::{
    instruction::Instruction,
    message::Message,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use stargaze_anchor::Config;
use stargaze_anchor_tests::{config_pda, ix_initialize, setup_svm};

fn send(
    svm: &mut litesvm::LiteSVM,
    payer: &Keypair,
    ixs: &[Instruction],
) -> Result<litesvm::types::TransactionMetadata, litesvm::types::FailedTransactionMetadata> {
    let blockhash = svm.latest_blockhash();
    let msg = Message::new(ixs, Some(&payer.pubkey()));
    let tx = Transaction::new(&[payer], msg, blockhash);
    svm.send_transaction(tx)
}

#[test]
fn sets_authority_and_zero_count() {
    let (mut svm, payer) = setup_svm();
    let authority = Pubkey::new_unique();

    send(
        &mut svm,
        &payer,
        &[ix_initialize(&payer.pubkey(), authority)],
    )
    .expect("initialize");

    let (config_addr, _) = config_pda();
    let cfg_acct = svm.get_account(&config_addr).expect("config exists");
    assert_eq!(cfg_acct.owner, stargaze_anchor::ID);
    let mut data = &cfg_acct.data[8..];
    let cfg = Config::deserialize(&mut data).expect("decode config");
    assert_eq!(cfg.authority, authority);
    assert_eq!(cfg.provider_count, 0);
}

#[test]
fn double_initialize_rejected() {
    let (mut svm, payer) = setup_svm();

    send(
        &mut svm,
        &payer,
        &[ix_initialize(&payer.pubkey(), payer.pubkey())],
    )
    .expect("first initialize");

    let err = send(
        &mut svm,
        &payer,
        &[ix_initialize(&payer.pubkey(), payer.pubkey())],
    )
    .expect_err("second initialize must fail");
    let _ = err;
}
