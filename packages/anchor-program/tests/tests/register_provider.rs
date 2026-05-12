use anchor_lang::AnchorDeserialize;
use solana_sdk::{
    message::Message,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use stargaze_anchor::{Config, Provider};
use stargaze_anchor_tests::{
    config_pda, ix_initialize, ix_register_provider, provider_pda, setup_svm,
};

const PROVIDER_ID: [u8; 32] = [1u8; 32];
const CATEGORY_HASH: [u8; 32] = [2u8; 32];
const META_CID: [u8; 32] = [3u8; 32];

fn send(svm: &mut litesvm::LiteSVM, payer: &Keypair, ixs: &[solana_sdk::instruction::Instruction]) -> Result<litesvm::types::TransactionMetadata, litesvm::types::FailedTransactionMetadata> {
    let blockhash = svm.latest_blockhash();
    let msg = Message::new(ixs, Some(&payer.pubkey()));
    let tx = Transaction::new(&[payer], msg, blockhash);
    svm.send_transaction(tx)
}

#[test]
fn happy_path() {
    let (mut svm, payer) = setup_svm();

    send(
        &mut svm,
        &payer,
        &[ix_initialize(&payer.pubkey(), payer.pubkey())],
    )
    .expect("initialize succeeds");

    send(
        &mut svm,
        &payer,
        &[ix_register_provider(
            &payer.pubkey(),
            PROVIDER_ID,
            CATEGORY_HASH,
            META_CID,
        )],
    )
    .expect("register_provider succeeds");

    let (provider_addr, _) = provider_pda(&PROVIDER_ID);
    let provider_acct = svm.get_account(&provider_addr).expect("provider exists");
    assert_eq!(provider_acct.owner, stargaze_anchor::ID);
    // skip 8-byte discriminator
    let mut data = &provider_acct.data[8..];
    let provider = Provider::deserialize(&mut data).expect("decode provider");
    assert_eq!(provider.owner, payer.pubkey());
    assert_eq!(provider.provider_id, PROVIDER_ID);
    assert_eq!(provider.category_hash, CATEGORY_HASH);
    assert_eq!(provider.meta_cid, META_CID);
    assert_eq!(provider.reputation_score, 500);
    assert!(provider.registered_at > 0);

    let (config_addr, _) = config_pda();
    let config_acct = svm.get_account(&config_addr).expect("config exists");
    let mut cfg_data = &config_acct.data[8..];
    let cfg = Config::deserialize(&mut cfg_data).expect("decode config");
    assert_eq!(cfg.provider_count, 1);
    assert_eq!(cfg.authority, payer.pubkey());
}

#[test]
fn duplicate_rejected() {
    let (mut svm, payer) = setup_svm();

    send(
        &mut svm,
        &payer,
        &[ix_initialize(&payer.pubkey(), payer.pubkey())],
    )
    .expect("initialize succeeds");

    send(
        &mut svm,
        &payer,
        &[ix_register_provider(
            &payer.pubkey(),
            PROVIDER_ID,
            CATEGORY_HASH,
            META_CID,
        )],
    )
    .expect("first registration succeeds");

    let err = send(
        &mut svm,
        &payer,
        &[ix_register_provider(
            &payer.pubkey(),
            PROVIDER_ID,
            CATEGORY_HASH,
            META_CID,
        )],
    )
    .expect_err("duplicate provider must be rejected");
    // The transaction must have failed. We don't pin the exact error code
    // (init-on-occupied yields a system-program / account-in-use failure).
    let _ = err;
}
