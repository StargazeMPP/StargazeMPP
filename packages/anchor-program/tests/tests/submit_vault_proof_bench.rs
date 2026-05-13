//! Compute-unit benchmark for `submit_vault_proof`. Drives one happy-path
//! proof through each currently-available verifier program
//! (`vault_verifier_aggregate_sum` — 1 public signal; `vault_verifier_geofence`
//! — 4 public signals) and prints `meta.compute_units_consumed`.
//!
//! Confirms the GROTH16_PLAN budget of <600k CU per ix. The
//! `buyer_key` verifier is a no-op stub today and rejects every input, so
//! there's no meaningful CU number to record for it.
//!
//! Numbers are recorded in `packages/anchor-program/BENCH.md`. Run via
//! `cargo test -p stargaze_anchor_tests --test submit_vault_proof_bench --
//! --nocapture`.

use solana_sdk::{
    instruction::Instruction,
    message::Message,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use stargaze_anchor::VaultTier;
use stargaze_anchor_tests::{
    compute_signals_hash, ix_configure_vault, ix_initialize, ix_register_provider,
    ix_submit_vault_proof, setup_svm_with_verifiers,
};

// Fixtures duplicated from `submit_vault_proof.rs` to keep this bench file
// self-contained.
const AGGREGATE_SUM_PROOF: [u8; 256] = [
    15, 94, 189, 196, 250, 135, 63, 120, 66, 154, 206, 209, 207, 9, 103, 101,
    33, 52, 161, 131, 252, 55, 15, 118, 88, 245, 200, 32, 195, 190, 50, 150,
    16, 210, 197, 242, 114, 3, 184, 136, 148, 205, 10, 202, 112, 56, 91, 220,
    243, 32, 153, 101, 37, 72, 181, 220, 94, 71, 181, 69, 234, 25, 2, 75,
    23, 88, 81, 147, 128, 139, 98, 215, 168, 53, 164, 223, 105, 51, 119, 60,
    148, 153, 49, 135, 193, 144, 176, 68, 227, 129, 119, 109, 239, 9, 214, 58,
    8, 206, 79, 71, 212, 66, 102, 55, 176, 95, 142, 53, 231, 210, 227, 86,
    182, 174, 138, 114, 19, 162, 25, 229, 34, 120, 172, 29, 15, 120, 89, 181,
    14, 116, 9, 165, 156, 223, 182, 168, 208, 209, 182, 128, 221, 85, 245, 160,
    112, 135, 42, 253, 51, 109, 72, 225, 106, 92, 82, 119, 8, 229, 167, 169,
    38, 167, 201, 96, 220, 218, 132, 107, 217, 218, 52, 245, 224, 243, 16, 30,
    71, 70, 157, 189, 252, 148, 183, 165, 36, 170, 170, 119, 134, 114, 24, 221,
    19, 61, 50, 61, 93, 233, 79, 200, 101, 181, 148, 113, 38, 135, 216, 230,
    57, 19, 196, 158, 1, 140, 10, 134, 162, 177, 163, 175, 129, 92, 138, 172,
    19, 9, 229, 10, 169, 241, 227, 102, 36, 48, 138, 36, 162, 187, 149, 60,
    125, 69, 184, 248, 174, 238, 9, 107, 74, 193, 1, 57, 79, 198, 156, 136,
];
const AGGREGATE_SUM_PUBLIC: [u8; 32] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    36,
];

const GEOFENCE_PROOF: [u8; 256] = [
    25, 117, 157, 160, 234, 43, 225, 77, 223, 168, 163, 139, 108, 212, 143, 162,
    73, 92, 90, 48, 161, 183, 204, 71, 251, 218, 133, 123, 17, 203, 221, 14,
    9, 244, 169, 49, 244, 46, 125, 68, 87, 132, 120, 172, 25, 97, 38, 131,
    18, 118, 139, 244, 49, 81, 116, 217, 83, 139, 13, 179, 114, 2, 191, 46,
    21, 124, 165, 129, 161, 123, 60, 21, 5, 160, 14, 209, 22, 12, 228, 177,
    150, 66, 30, 225, 136, 154, 234, 50, 111, 112, 93, 59, 36, 133, 209, 242,
    6, 253, 255, 164, 120, 183, 172, 71, 59, 167, 108, 20, 118, 182, 188, 72,
    239, 234, 188, 115, 152, 155, 188, 158, 71, 243, 238, 89, 187, 254, 75, 233,
    24, 58, 107, 113, 19, 102, 108, 249, 131, 234, 188, 243, 32, 63, 99, 29,
    93, 37, 48, 141, 244, 179, 227, 154, 107, 48, 42, 7, 206, 233, 33, 185,
    18, 193, 240, 174, 245, 152, 92, 166, 135, 14, 176, 232, 111, 137, 130, 14,
    207, 99, 43, 199, 1, 118, 201, 244, 165, 11, 115, 128, 92, 45, 224, 62,
    26, 200, 60, 211, 232, 254, 233, 118, 97, 149, 134, 188, 174, 133, 217, 192,
    5, 213, 233, 143, 190, 244, 85, 198, 74, 95, 68, 93, 130, 141, 39, 14,
    42, 88, 6, 174, 207, 141, 137, 218, 156, 136, 66, 174, 52, 234, 104, 48,
    234, 124, 119, 62, 62, 248, 97, 146, 24, 167, 9, 93, 210, 157, 38, 135,
];
const GEOFENCE_PUBLICS: [[u8; 32]; 4] = [
    [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 40,
    ],
    [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 70,
    ],
    [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 50,
    ],
    [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 80,
    ],
];

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

fn measure(
    label: &str,
    provider_id: [u8; 32],
    tier: VaultTier,
    verifier: Pubkey,
    proof: [u8; 256],
    signals: Vec<[u8; 32]>,
) -> u64 {
    let (mut svm, authority) = setup_svm_with_verifiers();

    send(
        &mut svm,
        &authority,
        &[&authority],
        &[ix_initialize(&authority.pubkey(), authority.pubkey())],
    )
    .expect("initialize");

    let provider = Keypair::new();
    svm.airdrop(&provider.pubkey(), 10_000_000_000)
        .expect("airdrop provider");
    send(
        &mut svm,
        &provider,
        &[&provider],
        &[ix_register_provider(
            &provider.pubkey(),
            provider_id,
            [0u8; 32],
            [0u8; 32],
        )],
    )
    .expect("register_provider");
    send(
        &mut svm,
        &provider,
        &[&provider],
        &[ix_configure_vault(
            &provider.pubkey(),
            provider_id,
            tier,
            verifier,
            [0u8; 32],
        )],
    )
    .expect("configure_vault");

    let submitter = Keypair::new();
    svm.airdrop(&submitter.pubkey(), 10_000_000_000)
        .expect("airdrop submitter");

    let signals_hash = compute_signals_hash(&signals);
    let meta = send(
        &mut svm,
        &submitter,
        &[&submitter],
        &[ix_submit_vault_proof(
            &submitter.pubkey(),
            verifier,
            provider_id,
            signals_hash,
            proof,
            signals,
        )],
    )
    .unwrap_or_else(|e| panic!("submit_vault_proof {label} failed: {:?}", e.err));

    meta.compute_units_consumed
}

#[test]
fn bench_submit_vault_proof_cu() {
    println!();
    println!("{:>15}  {:>10}", "circuit", "cu");

    let aggregate_cu = measure(
        "aggregate_sum",
        [0x01; 32],
        VaultTier::ZkAggregate,
        vault_verifier_aggregate_sum::ID,
        AGGREGATE_SUM_PROOF,
        vec![AGGREGATE_SUM_PUBLIC],
    );
    println!("{:>15}  {:>10}", "aggregate_sum", aggregate_cu);

    let geofence_cu = measure(
        "geofence",
        [0x02; 32],
        VaultTier::Confidential,
        vault_verifier_geofence::ID,
        GEOFENCE_PROOF,
        GEOFENCE_PUBLICS.to_vec(),
    );
    println!("{:>15}  {:>10}", "geofence", geofence_cu);
}
