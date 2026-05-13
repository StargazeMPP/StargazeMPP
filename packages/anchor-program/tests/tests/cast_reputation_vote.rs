use solana_sdk::{
    instruction::Instruction,
    message::Message,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use stargaze_anchor::{
    ReputationVoteBurned, ReputationVoted, COOLDOWN_DEFAULT_SECS, MIN_STAKE_DEFAULT,
    VERIFIED_STAKE_DEFAULT, VOTE_BURN_AMOUNT,
};
use stargaze_anchor_tests::{
    create_associated_token_account, create_mint, find_event, ix_cast_reputation_vote,
    ix_init_staking, ix_initialize, ix_register_provider, ix_set_stake_mint, mint_to, setup_svm,
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

/// Bring the SVM to a state where `cast_reputation_vote` will succeed:
/// initialise, register the provider, configure staking with a real mint,
/// create a voter with `VOTE_BURN_AMOUNT` of $GAZE in their ATA. Returns
/// the funded `(voter, mint)` pair.
fn bootstrap(svm: &mut litesvm::LiteSVM, authority: &Keypair, provider_id: [u8; 32]) -> (Keypair, Pubkey) {
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
        &[ix_register_provider(
            &authority.pubkey(),
            provider_id,
            [0u8; 32],
            [0u8; 32],
        )],
    )
    .expect("register");

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

    let voter = Keypair::new();
    svm.airdrop(&voter.pubkey(), 10_000_000_000)
        .expect("airdrop voter");
    let voter_ata = create_associated_token_account(svm, authority, &voter.pubkey(), &mint);
    mint_to(svm, authority, &mint, &voter_ata, authority, VOTE_BURN_AMOUNT * 5);

    (voter, mint)
}

#[test]
fn emits_vote_and_burn_events() {
    let (mut svm, authority) = setup_svm();
    let provider_id: [u8; 32] = [9u8; 32];
    let (voter, mint) = bootstrap(&mut svm, &authority, provider_id);

    let meta = send(
        &mut svm,
        &voter,
        &[&voter],
        &[ix_cast_reputation_vote(
            &voter.pubkey(),
            &mint,
            provider_id,
            true,
        )],
    )
    .expect("cast_reputation_vote");

    let vote: ReputationVoted = find_event(&meta.logs).expect("ReputationVoted event present");
    assert_eq!(vote.provider_id, provider_id);
    assert_eq!(vote.voter, voter.pubkey());
    assert!(vote.accurate);

    let burn: ReputationVoteBurned =
        find_event(&meta.logs).expect("ReputationVoteBurned event present");
    assert_eq!(burn.provider_id, provider_id);
    assert_eq!(burn.voter, voter.pubkey());
}

#[test]
fn rejects_when_stake_mint_unset() {
    // Setup without `set_stake_mint`: staking_config exists but stake_mint
    // is still the default Pubkey, which the handler should reject before
    // attempting the burn.
    let (mut svm, authority) = setup_svm();
    let provider_id: [u8; 32] = [10u8; 32];

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
            provider_id,
            [0u8; 32],
            [0u8; 32],
        )],
    )
    .expect("register");
    send(
        &mut svm,
        &authority,
        &[&authority],
        &[ix_init_staking(
            &authority.pubkey(),
            Pubkey::default(),
            MIN_STAKE_DEFAULT,
            VERIFIED_STAKE_DEFAULT,
            COOLDOWN_DEFAULT_SECS,
        )],
    )
    .expect("init_staking");

    // Use any pubkey as the "mint" — the constraint check fails first.
    let voter = Keypair::new();
    svm.airdrop(&voter.pubkey(), 10_000_000_000)
        .expect("airdrop voter");
    let fake_mint = Pubkey::new_unique();

    let err = send(
        &mut svm,
        &voter,
        &[&voter],
        &[ix_cast_reputation_vote(
            &voter.pubkey(),
            &fake_mint,
            provider_id,
            true,
        )],
    )
    .expect_err("cast_reputation_vote must reject when stake_mint unset");

    let logs = err.meta.logs.join("\n");
    // When stake_mint is unconfigured the caller has nothing to pass for
    // `stake_mint`. Anchor's account deserialization rejects the fake mint
    // before our explicit `StakeMintUnset` check fires — either failure mode
    // is acceptable, both prove the vote cannot happen without a real mint.
    assert!(
        logs.contains("StakeMintUnset")
            || logs.contains("AccountNotInitialized")
            || logs.contains("ConstraintRaw"),
        "expected mint configuration failure, got:\n{logs}"
    );
}
