use anchor_lang::AnchorDeserialize;
use solana_sdk::{
    instruction::Instruction,
    message::Message,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use stargaze_anchor::X402Receipt;
use stargaze_anchor_tests::{ix_initialize, ix_record_x402_receipt, setup_svm, x402_pda};

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

const SESSION_ID: [u8; 32] = [7u8; 32];
const PROVIDER_ID: [u8; 32] = [8u8; 32];
const AMOUNT: u64 = 1_234_567;

#[test]
fn pda_layout() {
    let (mut svm, payer) = setup_svm();

    send(
        &mut svm,
        &payer,
        &[ix_initialize(&payer.pubkey(), payer.pubkey())],
    )
    .expect("initialize");
    send(
        &mut svm,
        &payer,
        &[ix_record_x402_receipt(
            &payer.pubkey(),
            SESSION_ID,
            PROVIDER_ID,
            AMOUNT,
        )],
    )
    .expect("record receipt");

    let (receipt_addr, _) = x402_pda(&SESSION_ID, &PROVIDER_ID);
    let acct = svm.get_account(&receipt_addr).expect("receipt account exists");
    assert_eq!(acct.owner, stargaze_anchor::ID);
    // 8 (disc) + 32 (session_id) + 32 (provider_id) + 32 (payer) + 8 (amount)
    // + 8 (paid_at) + 1 (bump) = 121.
    assert_eq!(acct.data.len(), 121);

    let mut data = &acct.data[8..];
    let receipt = X402Receipt::deserialize(&mut data).expect("decode receipt");
    assert_eq!(receipt.session_id, SESSION_ID);
    assert_eq!(receipt.provider_id, PROVIDER_ID);
    assert_eq!(receipt.payer, payer.pubkey());
    assert_eq!(receipt.amount, AMOUNT);
    assert!(receipt.paid_at > 0);
}
