//! LiteSVM tests for the `vault_verifier_buyer_key` stub program.
//!
//! The BuyerKey circuit is not yet finalised. The program ships as a registered
//! verifier id so providers can configure `VaultConfig.on_chain_verifier` today,
//! but every call to `verify` returns `CircuitNotFinalised` until a real
//! circuit + vkey land.

use anchor_lang::InstructionData;
use solana_sdk::{
    instruction::Instruction,
    message::Message,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use stargaze_anchor_tests::{setup_verifier_svm, VAULT_VERIFIER_BUYER_KEY_SO};

fn verify_ix(proof_bytes: [u8; 256], public_signals: Vec<[u8; 32]>) -> Instruction {
    let data = vault_verifier_buyer_key::instruction::Verify {
        proof_bytes,
        public_signals,
    }
    .data();
    Instruction {
        program_id: vault_verifier_buyer_key::ID,
        accounts: vec![],
        data,
    }
}

fn send(
    svm: &mut litesvm::LiteSVM,
    payer: &Keypair,
    ix: Instruction,
) -> Result<litesvm::types::TransactionMetadata, litesvm::types::FailedTransactionMetadata> {
    let blockhash = svm.latest_blockhash();
    let msg = Message::new(&[ix], Some(&payer.pubkey()));
    let tx = Transaction::new(&[payer], msg, blockhash);
    svm.send_transaction(tx)
}

#[test]
fn verify_always_rejects() {
    let (mut svm, payer) = setup_verifier_svm(
        vault_verifier_buyer_key::ID,
        VAULT_VERIFIER_BUYER_KEY_SO,
    );

    let err = send(&mut svm, &payer, verify_ix([0u8; 256], vec![[0u8; 32]; 1]))
        .expect_err("stub must always reject");
    // CircuitNotFinalised is the first (and only) variant in ErrorCode → 6000.
    let msg = format!("{:?}", err.err);
    assert!(
        msg.contains("Custom(6000)"),
        "expected CircuitNotFinalised (Custom(6000)), got: {msg}"
    );
}
