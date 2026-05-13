//! Integration tests for `dispatch_stake_to_tempo`.
//!
//! Mirrors the shape of `dispatch_reputation.rs` but reads from the per-staker
//! `StakeAccount` rather than the `Provider.reputation_score`. The payload is
//! `abi.encode(bytes32 providerId, address owner, uint256 amount)` — 96 bytes
//! total — and we assert the byte layout exactly so the Tempo decoder stays
//! in lockstep.

use solana_sdk::{
    instruction::Instruction,
    message::Message,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use stargaze_anchor::{
    StakeDispatched, COOLDOWN_DEFAULT_SECS, MIN_STAKE_DEFAULT, VERIFIED_STAKE_DEFAULT,
};
use stargaze_anchor_tests::{
    create_associated_token_account, create_mint, find_event, ix_claim_unstake,
    ix_dispatch_stake_to_tempo, ix_init_staking, ix_initialize, ix_request_unstake,
    ix_set_stake_mint, ix_stake, mint_to, setup_svm, warp_clock,
};

const PROVIDER_ID: [u8; 32] = [7u8; 32];
const TEMPO_RECEIVER: [u8; 20] = [
    0xde, 0xad, 0xbe, 0xef, 0xfe, 0xed, 0xfa, 0xce, 0xca, 0xfe, 0xba, 0xbe, 0xc0, 0x01, 0xb0, 0x0b,
    0x10, 0x10, 0xab, 0xcd,
];
const DEST_SELECTOR: u64 = 16_015_286_601_757_825_753;

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

/// Bring the staking system up to a usable state with a real mint and a seeded
/// staker. Returns `(mint_kp, staker, staker_ata)` — same shape as the
/// `bootstrap_with_mint` helper in `staking.rs`.
fn bootstrap_with_mint(
    svm: &mut litesvm::LiteSVM,
    authority: &Keypair,
    initial_balance: u64,
) -> (Keypair, Keypair, Pubkey) {
    send(
        svm,
        authority,
        &[authority],
        &[ix_initialize(&authority.pubkey(), authority.pubkey())],
    )
    .expect("initialize");

    send(
        svm,
        authority,
        &[authority],
        &[ix_init_staking(
            &authority.pubkey(),
            Pubkey::default(),
            MIN_STAKE_DEFAULT,
            VERIFIED_STAKE_DEFAULT,
            COOLDOWN_DEFAULT_SECS,
        )],
    )
    .expect("init_staking");

    let mint_kp = create_mint(svm, authority, &authority.pubkey(), 6);
    let mint = mint_kp.pubkey();

    send(
        svm,
        authority,
        &[authority],
        &[ix_set_stake_mint(&authority.pubkey(), mint)],
    )
    .expect("set_stake_mint");

    let staker = Keypair::new();
    svm.airdrop(&staker.pubkey(), 10_000_000_000)
        .expect("airdrop staker");
    let staker_ata = create_associated_token_account(svm, authority, &staker.pubkey(), &mint);
    mint_to(svm, authority, &mint, &staker_ata, authority, initial_balance);

    (mint_kp, staker, staker_ata)
}

/// Expected 96-byte ABI-encoded payload:
///   bytes  0..32  — provider_id (already 32 bytes)
///   bytes 32..44  — 12 zero pad bytes (Solidity address left padding)
///   bytes 44..64  — owner.to_bytes()[12..32] (bottom 20 bytes of Solana pubkey)
///   bytes 64..88  — 24 zero pad bytes (uint256 left padding)
///   bytes 88..96  — amount.to_be_bytes() (u64 big-endian)
fn expected_payload(provider_id: [u8; 32], owner: &Pubkey, amount: u64) -> Vec<u8> {
    let mut out = Vec::with_capacity(96);
    out.extend_from_slice(&provider_id);
    out.extend_from_slice(&[0u8; 12]);
    let owner_bytes = owner.to_bytes();
    out.extend_from_slice(&owner_bytes[12..32]);
    out.extend_from_slice(&[0u8; 24]);
    out.extend_from_slice(&amount.to_be_bytes());
    out
}

#[test]
fn dispatches_with_amount() {
    let (mut svm, authority) = setup_svm();
    let ccip_router = Pubkey::new_unique();

    let initial = 5 * MIN_STAKE_DEFAULT;
    let (mint_kp, staker, _staker_ata) = bootstrap_with_mint(&mut svm, &authority, initial);
    let mint = mint_kp.pubkey();

    // Stake exactly 100 GAZE (100 * 10^6 base units) so the test asserts a
    // distinct, easy-to-eyeball amount on the wire.
    let stake_amount: u64 = 100_000_000;
    send(
        &mut svm,
        &staker,
        &[&staker],
        &[ix_stake(&staker.pubkey(), &mint, PROVIDER_ID, stake_amount)],
    )
    .expect("stake");

    let meta = send(
        &mut svm,
        &authority,
        &[&authority],
        &[ix_dispatch_stake_to_tempo(
            &authority.pubkey(),
            &ccip_router,
            PROVIDER_ID,
            staker.pubkey(),
            DEST_SELECTOR,
            TEMPO_RECEIVER.to_vec(),
            vec![],
        )],
    )
    .expect("dispatch_stake_to_tempo");

    let evt: StakeDispatched = find_event(&meta.logs).expect("StakeDispatched event present");
    assert_eq!(evt.provider_id, PROVIDER_ID);
    assert_eq!(evt.owner, staker.pubkey());
    assert_eq!(evt.amount, 100_000_000);
    assert_eq!(evt.dest_chain_selector, DEST_SELECTOR);
    assert_eq!(evt.receiver, TEMPO_RECEIVER.to_vec());

    // Byte-for-byte payload check — the Tempo decoder is parameterised on
    // exactly this layout.
    let want = expected_payload(PROVIDER_ID, &staker.pubkey(), 100_000_000);
    assert_eq!(evt.payload.len(), 96);
    assert_eq!(evt.payload, want);

    // Sanity-check the individual slots so a regression surfaces with a
    // useful failure message rather than a giant byte-array diff.
    assert_eq!(&evt.payload[0..32], &PROVIDER_ID);
    assert!(evt.payload[32..44].iter().all(|b| *b == 0));
    assert_eq!(&evt.payload[44..64], &staker.pubkey().to_bytes()[12..32]);
    assert!(evt.payload[64..88].iter().all(|b| *b == 0));
    assert_eq!(
        u64::from_be_bytes(evt.payload[88..96].try_into().unwrap()),
        100_000_000,
    );
}

#[test]
fn dispatches_zero_when_no_stake() {
    let (mut svm, authority) = setup_svm();
    let ccip_router = Pubkey::new_unique();

    let initial = 4 * MIN_STAKE_DEFAULT;
    let (mint_kp, staker, _staker_ata) = bootstrap_with_mint(&mut svm, &authority, initial);
    let mint = mint_kp.pubkey();

    // Stake -> request_unstake the full amount -> warp -> claim. After this
    // the StakeAccount still exists but `amount == 0`, which is the natural
    // post-claim state of a staker who has fully exited a single provider.
    let stake_amount = 2 * MIN_STAKE_DEFAULT;
    send(
        &mut svm,
        &staker,
        &[&staker],
        &[ix_stake(&staker.pubkey(), &mint, PROVIDER_ID, stake_amount)],
    )
    .expect("stake");
    send(
        &mut svm,
        &staker,
        &[&staker],
        &[ix_request_unstake(&staker.pubkey(), PROVIDER_ID, stake_amount)],
    )
    .expect("request_unstake");
    warp_clock(&mut svm, COOLDOWN_DEFAULT_SECS + 1);
    svm.expire_blockhash();
    send(
        &mut svm,
        &staker,
        &[&staker],
        &[ix_claim_unstake(&staker.pubkey(), &mint, PROVIDER_ID)],
    )
    .expect("claim_unstake");

    let meta = send(
        &mut svm,
        &authority,
        &[&authority],
        &[ix_dispatch_stake_to_tempo(
            &authority.pubkey(),
            &ccip_router,
            PROVIDER_ID,
            staker.pubkey(),
            DEST_SELECTOR,
            TEMPO_RECEIVER.to_vec(),
            vec![],
        )],
    )
    .expect("dispatch_stake_to_tempo (zero amount)");

    let evt: StakeDispatched = find_event(&meta.logs).expect("StakeDispatched event present");
    assert_eq!(evt.amount, 0);
    assert_eq!(evt.owner, staker.pubkey());
    assert_eq!(evt.provider_id, PROVIDER_ID);

    // Payload must still be a well-formed 96 bytes — just with a zero amount
    // suffix.
    assert_eq!(evt.payload.len(), 96);
    let want = expected_payload(PROVIDER_ID, &staker.pubkey(), 0);
    assert_eq!(evt.payload, want);
}

#[test]
fn rejects_non_admin() {
    let (mut svm, authority) = setup_svm();
    let ccip_router = Pubkey::new_unique();

    let initial = 3 * MIN_STAKE_DEFAULT;
    let (mint_kp, staker, _staker_ata) = bootstrap_with_mint(&mut svm, &authority, initial);
    let mint = mint_kp.pubkey();

    let stake_amount = MIN_STAKE_DEFAULT;
    send(
        &mut svm,
        &staker,
        &[&staker],
        &[ix_stake(&staker.pubkey(), &mint, PROVIDER_ID, stake_amount)],
    )
    .expect("stake");

    let attacker = Keypair::new();
    svm.airdrop(&attacker.pubkey(), 1_000_000_000)
        .expect("airdrop attacker");

    let err = send(
        &mut svm,
        &attacker,
        &[&attacker],
        &[ix_dispatch_stake_to_tempo(
            &attacker.pubkey(),
            &ccip_router,
            PROVIDER_ID,
            staker.pubkey(),
            DEST_SELECTOR,
            TEMPO_RECEIVER.to_vec(),
            vec![],
        )],
    )
    .expect_err("non-authority must be rejected");

    let logs = err.meta.logs.join("\n");
    assert!(
        logs.contains("Unauthorized") || logs.contains("not authorised"),
        "expected Unauthorized in logs, got:\n{logs}"
    );
}
