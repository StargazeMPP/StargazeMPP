//! Integration tests for `submit_vault_proof` on `stargaze_anchor`.
//!
//! Exercises the manual CPI from stargaze_anchor into the per-circuit
//! Groth16 verifier programs (`vault_verifier_aggregate_sum`,
//! `vault_verifier_geofence`, `vault_verifier_buyer_key`). Fixtures are
//! copied from the per-circuit verifier tests so a single SVM run can
//! drive both circuits without rebuilding the proof scripts.

use anchor_lang::AnchorDeserialize;
use solana_sdk::{
    instruction::Instruction,
    message::Message,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use stargaze_anchor::{VaultProofRecord, VaultProofVerified, VaultTier};
use stargaze_anchor_tests::{
    compute_signals_hash, find_event, ix_configure_vault, ix_initialize, ix_register_provider,
    ix_submit_vault_proof, setup_svm_with_verifiers, vault_proof_pda,
};

// === fixture: aggregate_sum (N=8, claimedSum=36) ===
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

// === fixture: geofence (N=32 bit-width, 4 public signals) ===
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

const PROVIDER_ID: [u8; 32] = [0x11; 32];
const CATEGORY_HASH: [u8; 32] = [0x22; 32];
const META_CID: [u8; 32] = [0x33; 32];
const ARWEAVE_CID: [u8; 32] = [0x44; 32];

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

fn bootstrap(svm: &mut litesvm::LiteSVM, authority: &Keypair) -> Keypair {
    send(
        svm,
        authority,
        &[authority],
        &[ix_initialize(&authority.pubkey(), authority.pubkey())],
    )
    .expect("initialize");

    let provider = Keypair::new();
    svm.airdrop(&provider.pubkey(), 10_000_000_000)
        .expect("airdrop provider");
    send(
        svm,
        &provider,
        &[&provider],
        &[ix_register_provider(
            &provider.pubkey(),
            PROVIDER_ID,
            CATEGORY_HASH,
            META_CID,
        )],
    )
    .expect("register provider");
    provider
}

fn configure(svm: &mut litesvm::LiteSVM, provider: &Keypair, tier: VaultTier, verifier: Pubkey) {
    send(
        svm,
        provider,
        &[provider],
        &[ix_configure_vault(
            &provider.pubkey(),
            PROVIDER_ID,
            tier,
            verifier,
            ARWEAVE_CID,
        )],
    )
    .unwrap_or_else(|_| panic!("configure_vault {tier:?}"));
}

fn read_record(svm: &litesvm::LiteSVM, provider_id: &[u8; 32], signals_hash: &[u8; 32]) -> VaultProofRecord {
    let (addr, _) = vault_proof_pda(provider_id, signals_hash);
    let acct = svm.get_account(&addr).expect("vault proof record exists");
    let mut data = &acct.data[8..];
    VaultProofRecord::deserialize(&mut data).expect("decode vault proof record")
}

#[test]
fn submit_vault_proof_happy_path_aggregate_sum() {
    let (mut svm, authority) = setup_svm_with_verifiers();
    let provider = bootstrap(&mut svm, &authority);
    configure(
        &mut svm,
        &provider,
        VaultTier::ZkAggregate,
        vault_verifier_aggregate_sum::ID,
    );

    let signals = vec![AGGREGATE_SUM_PUBLIC];
    let signals_hash = compute_signals_hash(&signals);

    let submitter = Keypair::new();
    svm.airdrop(&submitter.pubkey(), 10_000_000_000)
        .expect("airdrop submitter");

    let meta = send(
        &mut svm,
        &submitter,
        &[&submitter],
        &[ix_submit_vault_proof(
            &submitter.pubkey(),
            vault_verifier_aggregate_sum::ID,
            PROVIDER_ID,
            signals_hash,
            AGGREGATE_SUM_PROOF,
            signals,
        )],
    )
    .expect("submit_vault_proof aggregate_sum");

    let record = read_record(&svm, &PROVIDER_ID, &signals_hash);
    assert_eq!(record.provider_id, PROVIDER_ID);
    assert_eq!(record.signals_hash, signals_hash);
    assert_eq!(record.submitter, submitter.pubkey());

    let event: VaultProofVerified =
        find_event(&meta.logs).expect("VaultProofVerified emitted");
    assert_eq!(event.provider_id, PROVIDER_ID);
    assert_eq!(event.tier, VaultTier::ZkAggregate);
    assert_eq!(event.signals_hash, signals_hash);
    assert_eq!(event.submitter, submitter.pubkey());
    assert_eq!(event.slot, record.slot);
}

#[test]
fn submit_vault_proof_happy_path_geofence() {
    let (mut svm, authority) = setup_svm_with_verifiers();
    let provider = bootstrap(&mut svm, &authority);
    configure(
        &mut svm,
        &provider,
        VaultTier::Confidential,
        vault_verifier_geofence::ID,
    );

    let signals = GEOFENCE_PUBLICS.to_vec();
    let signals_hash = compute_signals_hash(&signals);

    let submitter = Keypair::new();
    svm.airdrop(&submitter.pubkey(), 10_000_000_000)
        .expect("airdrop submitter");

    let meta = send(
        &mut svm,
        &submitter,
        &[&submitter],
        &[ix_submit_vault_proof(
            &submitter.pubkey(),
            vault_verifier_geofence::ID,
            PROVIDER_ID,
            signals_hash,
            GEOFENCE_PROOF,
            signals,
        )],
    )
    .expect("submit_vault_proof geofence");

    let event: VaultProofVerified =
        find_event(&meta.logs).expect("VaultProofVerified emitted");
    assert_eq!(event.tier, VaultTier::Confidential);
    assert_eq!(event.signals_hash, signals_hash);
}

#[test]
fn submit_vault_proof_rejects_buyer_key_stub() {
    let (mut svm, authority) = setup_svm_with_verifiers();
    let provider = bootstrap(&mut svm, &authority);
    configure(
        &mut svm,
        &provider,
        VaultTier::BuyerKey,
        vault_verifier_buyer_key::ID,
    );

    // The stub rejects regardless of inputs; we pass a syntactically valid
    // single-signal payload just so signals_hash committment passes the
    // pre-CPI check and we exercise the CPI -> ProofVerificationFailed path.
    let signals = vec![[0u8; 32]];
    let signals_hash = compute_signals_hash(&signals);

    let submitter = Keypair::new();
    svm.airdrop(&submitter.pubkey(), 10_000_000_000)
        .expect("airdrop submitter");

    let err = send(
        &mut svm,
        &submitter,
        &[&submitter],
        &[ix_submit_vault_proof(
            &submitter.pubkey(),
            vault_verifier_buyer_key::ID,
            PROVIDER_ID,
            signals_hash,
            [0u8; 256],
            signals,
        )],
    )
    .expect_err("buyer_key stub must reject");

    let logs = err.meta.logs.join("\n");
    assert!(
        logs.contains("CircuitNotFinalised") || logs.contains("ProofVerificationFailed"),
        "expected stub or wrapper rejection, got:\n{logs}"
    );
}

#[test]
fn submit_vault_proof_rejects_replay() {
    let (mut svm, authority) = setup_svm_with_verifiers();
    let provider = bootstrap(&mut svm, &authority);
    configure(
        &mut svm,
        &provider,
        VaultTier::ZkAggregate,
        vault_verifier_aggregate_sum::ID,
    );

    let signals = vec![AGGREGATE_SUM_PUBLIC];
    let signals_hash = compute_signals_hash(&signals);
    let submitter = Keypair::new();
    svm.airdrop(&submitter.pubkey(), 10_000_000_000)
        .expect("airdrop submitter");

    let make_ix = || {
        ix_submit_vault_proof(
            &submitter.pubkey(),
            vault_verifier_aggregate_sum::ID,
            PROVIDER_ID,
            signals_hash,
            AGGREGATE_SUM_PROOF,
            signals.clone(),
        )
    };

    send(&mut svm, &submitter, &[&submitter], &[make_ix()])
        .expect("first submission succeeds");

    // Roll the blockhash so litesvm doesn't reject the second tx as a
    // duplicate before the program has a chance to surface the replay.
    svm.expire_blockhash();
    let err = send(&mut svm, &submitter, &[&submitter], &[make_ix()])
        .expect_err("second submission with same signals_hash must fail");

    // Anchor's `init` constraint fails when the PDA already has lamports —
    // the System Program reports `account already in use` (0x0).
    let logs = err.meta.logs.join("\n");
    assert!(
        logs.contains("already in use") || logs.contains("custom program error: 0x0"),
        "expected `account already in use` system-program rejection, got:\n{logs}"
    );
}

#[test]
fn submit_vault_proof_rejects_wrong_verifier_program() {
    let (mut svm, authority) = setup_svm_with_verifiers();
    let provider = bootstrap(&mut svm, &authority);
    configure(
        &mut svm,
        &provider,
        VaultTier::ZkAggregate,
        vault_verifier_aggregate_sum::ID,
    );

    let signals = vec![AGGREGATE_SUM_PUBLIC];
    let signals_hash = compute_signals_hash(&signals);
    let submitter = Keypair::new();
    svm.airdrop(&submitter.pubkey(), 10_000_000_000)
        .expect("airdrop submitter");

    // Vault is wired to aggregate_sum but the caller passes geofence's id.
    let err = send(
        &mut svm,
        &submitter,
        &[&submitter],
        &[ix_submit_vault_proof(
            &submitter.pubkey(),
            vault_verifier_geofence::ID,
            PROVIDER_ID,
            signals_hash,
            AGGREGATE_SUM_PROOF,
            signals,
        )],
    )
    .expect_err("wrong verifier program must be rejected");

    let logs = err.meta.logs.join("\n");
    assert!(
        logs.contains("VerifierProgramMismatch"),
        "expected VerifierProgramMismatch in logs, got:\n{logs}"
    );
}

#[test]
fn submit_vault_proof_rejects_open_tier() {
    let (mut svm, authority) = setup_svm_with_verifiers();
    let provider = bootstrap(&mut svm, &authority);
    configure(
        &mut svm,
        &provider,
        VaultTier::Open,
        vault_verifier_aggregate_sum::ID,
    );

    let signals = vec![AGGREGATE_SUM_PUBLIC];
    let signals_hash = compute_signals_hash(&signals);
    let submitter = Keypair::new();
    svm.airdrop(&submitter.pubkey(), 10_000_000_000)
        .expect("airdrop submitter");

    let err = send(
        &mut svm,
        &submitter,
        &[&submitter],
        &[ix_submit_vault_proof(
            &submitter.pubkey(),
            vault_verifier_aggregate_sum::ID,
            PROVIDER_ID,
            signals_hash,
            AGGREGATE_SUM_PROOF,
            signals,
        )],
    )
    .expect_err("Open tier must not accept proofs");

    let logs = err.meta.logs.join("\n");
    assert!(
        logs.contains("TierDoesNotRequireProof"),
        "expected TierDoesNotRequireProof in logs, got:\n{logs}"
    );
}

#[test]
fn submit_vault_proof_rejects_unset_verifier() {
    let (mut svm, authority) = setup_svm_with_verifiers();
    let provider = bootstrap(&mut svm, &authority);
    // Configure a non-Open tier but leave on_chain_verifier as the default
    // sentinel. submit_vault_proof must reject before any CPI dispatch.
    configure(
        &mut svm,
        &provider,
        VaultTier::ZkAggregate,
        Pubkey::default(),
    );

    let signals = vec![AGGREGATE_SUM_PUBLIC];
    let signals_hash = compute_signals_hash(&signals);
    let submitter = Keypair::new();
    svm.airdrop(&submitter.pubkey(), 10_000_000_000)
        .expect("airdrop submitter");

    let err = send(
        &mut svm,
        &submitter,
        &[&submitter],
        &[ix_submit_vault_proof(
            &submitter.pubkey(),
            // Any non-default key — the handler checks `VerifierUnset` on
            // the vault first, so this never gets CPI'd.
            vault_verifier_aggregate_sum::ID,
            PROVIDER_ID,
            signals_hash,
            AGGREGATE_SUM_PROOF,
            signals,
        )],
    )
    .expect_err("unset verifier must be rejected");

    let logs = err.meta.logs.join("\n");
    assert!(
        logs.contains("VerifierUnset"),
        "expected VerifierUnset in logs, got:\n{logs}"
    );
}

#[test]
fn submit_vault_proof_rejects_signals_hash_mismatch() {
    let (mut svm, authority) = setup_svm_with_verifiers();
    let provider = bootstrap(&mut svm, &authority);
    configure(
        &mut svm,
        &provider,
        VaultTier::ZkAggregate,
        vault_verifier_aggregate_sum::ID,
    );

    let signals = vec![AGGREGATE_SUM_PUBLIC];
    // Deliberately wrong commitment — keeps the PDA derivation consistent
    // (so the account constraint passes) while the handler's recomputed
    // hash diverges from the caller-supplied one.
    let wrong_hash = [0xCC; 32];
    let submitter = Keypair::new();
    svm.airdrop(&submitter.pubkey(), 10_000_000_000)
        .expect("airdrop submitter");

    let err = send(
        &mut svm,
        &submitter,
        &[&submitter],
        &[ix_submit_vault_proof(
            &submitter.pubkey(),
            vault_verifier_aggregate_sum::ID,
            PROVIDER_ID,
            wrong_hash,
            AGGREGATE_SUM_PROOF,
            signals,
        )],
    )
    .expect_err("mismatched signals_hash must be rejected");

    let logs = err.meta.logs.join("\n");
    assert!(
        logs.contains("SignalsHashMismatch"),
        "expected SignalsHashMismatch in logs, got:\n{logs}"
    );
}
