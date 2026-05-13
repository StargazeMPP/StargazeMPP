//! Integration test for [`PostgresSink`] — writes one of every
//! [`DecodedEvent`] variant against a real Postgres instance, then reads
//! each row back and asserts every column was bound to the expected
//! value. Also exercises the `ON CONFLICT (slot, signature) DO NOTHING`
//! idempotency invariant by replaying every write a second time.
//!
//! Gated by `#[ignore]` because it requires a `DATABASE_URL` reachable
//! Postgres instance — CI without a database must not run it.
//!
//! # Run against a local Postgres (operator-provided):
//!
//! ```text
//! DATABASE_URL=postgres://user:pass@localhost:5432/stargaze_test \
//!   cargo test -p stargaze-indexer --test postgres_sink -- --ignored
//! ```
//!
//! Slots used by the test sit in `[100_000, 200_000)` so concurrent runs
//! against a shared database don't collide with real ingestion data, and
//! a pre-clean `DELETE` at the top of the test wipes that range out of
//! every projected table before writing the fixtures.

use sqlx::postgres::PgPoolOptions;
use sqlx::Row;

use stargaze_events::{
    DecodedEvent, EscrowInitialized, ProviderRegistered, PubkeyBytes, ReputationScoreSet,
    ReputationVoteBurned, ReputationVoted, RoutingFeeProcessed, SessionOpened, SessionSettled,
    Slashed, StakeMintSet, Staked, StakingInitialized, UnstakeRequested, Unstaked,
    VaultAuditorKeySet, VaultBuyerKeyRotationUpdated, VaultConfigured, VaultDeactivated,
    VaultProofVerified, VaultTier, VoucherSettled, X402ReceiptRecorded,
};
use stargaze_indexer::sink::{EventSink, PostgresSink};

/// All projection tables — the pre-clean step deletes the test slot
/// range from every one of them before the fixtures are inserted.
const ALL_TABLES: &[&str] = &[
    "provider_registered",
    "reputation_voted",
    "x402_receipt_recorded",
    "staked",
    "unstake_requested",
    "unstaked",
    "slashed",
    "staking_initialized",
    "stake_mint_set",
    "routing_fee_processed",
    "reputation_vote_burned",
    "vault_proof_verified",
    "reputation_score_set",
    "escrow_initialized",
    "session_opened",
    "voucher_settled",
    "session_settled",
    "vault_configured",
    "vault_auditor_key_set",
    "vault_buyer_key_rotation_updated",
    "vault_deactivated",
];

const SLOT_BASE: i64 = 100_000;

#[tokio::test]
#[ignore]
async fn writes_every_event_variant() {
    let url = std::env::var("DATABASE_URL").expect(
        "DATABASE_URL must be set to a reachable Postgres instance — this test is #[ignore]d \
         so CI without a database does not run it; invoke explicitly with \
         `cargo test -p stargaze-indexer --test postgres_sink -- --ignored`.",
    );

    // Connect via the sink — this runs both bundled migrations.
    let sink = PostgresSink::connect(&url)
        .await
        .expect("PostgresSink::connect must succeed");

    // Second pool for read-back assertions (the sink doesn't expose its pool).
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .expect("read-back pool connects");

    // Pre-clean: wipe the test slot range from every table so concurrent
    // runs of this test don't trip the UNIQUE (slot, signature) constraint
    // on the second pass.
    for table in ALL_TABLES {
        let sql = format!(
            "DELETE FROM {} WHERE slot >= 100000 AND slot < 200000",
            table
        );
        sqlx::query(&sql)
            .execute(&pool)
            .await
            .unwrap_or_else(|e| panic!("pre-clean DELETE on {} failed: {}", table, e));
    }

    // -------- 1. ProviderRegistered --------
    {
        let slot = SLOT_BASE + 1;
        let sig = "test-sig-1";
        let event = DecodedEvent::ProviderRegistered(ProviderRegistered {
            provider_id: [0x11u8; 32],
            owner: PubkeyBytes([0x12u8; 32]),
            category_hash: [0x13u8; 32],
            meta_cid: [0x14u8; 32],
        });
        sink.write(slot as u64, Some(sig), &event).await.unwrap();

        let row = sqlx::query(
            "SELECT slot, signature, provider_id, owner, category_hash, meta_cid \
             FROM provider_registered WHERE slot = $1",
        )
        .bind(slot)
        .fetch_one(&pool)
        .await
        .expect("provider_registered row exists");
        assert_eq!(row.get::<i64, _>("slot"), slot);
        assert_eq!(row.get::<String, _>("signature"), sig);
        assert_eq!(row.get::<Vec<u8>, _>("provider_id"), vec![0x11u8; 32]);
        assert_eq!(row.get::<Vec<u8>, _>("owner"), vec![0x12u8; 32]);
        assert_eq!(row.get::<Vec<u8>, _>("category_hash"), vec![0x13u8; 32]);
        assert_eq!(row.get::<Vec<u8>, _>("meta_cid"), vec![0x14u8; 32]);

        // Idempotency.
        sink.write(slot as u64, Some(sig), &event).await.unwrap();
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM provider_registered WHERE slot = $1",
        )
        .bind(slot)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(count, 1, "ON CONFLICT DO NOTHING must hold (provider_registered)");
    }

    // -------- 2. ReputationVoted --------
    {
        let slot = SLOT_BASE + 2;
        let sig = "test-sig-2";
        let event = DecodedEvent::ReputationVoted(ReputationVoted {
            provider_id: [0x21u8; 32],
            voter: PubkeyBytes([0x22u8; 32]),
            accurate: true,
        });
        sink.write(slot as u64, Some(sig), &event).await.unwrap();

        let row = sqlx::query(
            "SELECT slot, signature, provider_id, voter, accurate \
             FROM reputation_voted WHERE slot = $1",
        )
        .bind(slot)
        .fetch_one(&pool)
        .await
        .expect("reputation_voted row exists");
        assert_eq!(row.get::<i64, _>("slot"), slot);
        assert_eq!(row.get::<String, _>("signature"), sig);
        assert_eq!(row.get::<Vec<u8>, _>("provider_id"), vec![0x21u8; 32]);
        assert_eq!(row.get::<Vec<u8>, _>("voter"), vec![0x22u8; 32]);
        assert!(row.get::<bool, _>("accurate"));

        sink.write(slot as u64, Some(sig), &event).await.unwrap();
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM reputation_voted WHERE slot = $1",
        )
        .bind(slot)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(count, 1, "ON CONFLICT DO NOTHING must hold (reputation_voted)");
    }

    // -------- 3. X402ReceiptRecorded --------
    {
        let slot = SLOT_BASE + 3;
        let sig = "test-sig-3";
        let amount: u64 = 1_234_567;
        let paid_at: i64 = 1_700_000_000;
        let event = DecodedEvent::X402ReceiptRecorded(X402ReceiptRecorded {
            session_id: [0x31u8; 32],
            provider_id: [0x32u8; 32],
            payer: PubkeyBytes([0x33u8; 32]),
            amount,
            paid_at,
        });
        sink.write(slot as u64, Some(sig), &event).await.unwrap();

        let row = sqlx::query(
            "SELECT slot, signature, session_id, provider_id, payer, amount, paid_at \
             FROM x402_receipt_recorded WHERE slot = $1",
        )
        .bind(slot)
        .fetch_one(&pool)
        .await
        .expect("x402_receipt_recorded row exists");
        assert_eq!(row.get::<i64, _>("slot"), slot);
        assert_eq!(row.get::<String, _>("signature"), sig);
        assert_eq!(row.get::<Vec<u8>, _>("session_id"), vec![0x31u8; 32]);
        assert_eq!(row.get::<Vec<u8>, _>("provider_id"), vec![0x32u8; 32]);
        assert_eq!(row.get::<Vec<u8>, _>("payer"), vec![0x33u8; 32]);
        assert_eq!(row.get::<i64, _>("amount"), amount as i64);
        assert_eq!(row.get::<i64, _>("paid_at"), paid_at);

        sink.write(slot as u64, Some(sig), &event).await.unwrap();
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM x402_receipt_recorded WHERE slot = $1",
        )
        .bind(slot)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(count, 1, "ON CONFLICT DO NOTHING must hold (x402_receipt_recorded)");
    }

    // -------- 6. Staked --------
    {
        let slot = SLOT_BASE + 6;
        let sig = "test-sig-6";
        let amount: u64 = 50_000_000;
        let total: u64 = 150_000_000;
        let event = DecodedEvent::Staked(Staked {
            provider_id: [0x61u8; 32],
            owner: PubkeyBytes([0x62u8; 32]),
            amount,
            total,
        });
        sink.write(slot as u64, Some(sig), &event).await.unwrap();

        let row = sqlx::query(
            "SELECT slot, signature, provider_id, owner, amount, total \
             FROM staked WHERE slot = $1",
        )
        .bind(slot)
        .fetch_one(&pool)
        .await
        .expect("staked row exists");
        assert_eq!(row.get::<i64, _>("slot"), slot);
        assert_eq!(row.get::<String, _>("signature"), sig);
        assert_eq!(row.get::<Vec<u8>, _>("provider_id"), vec![0x61u8; 32]);
        assert_eq!(row.get::<Vec<u8>, _>("owner"), vec![0x62u8; 32]);
        assert_eq!(row.get::<i64, _>("amount"), amount as i64);
        assert_eq!(row.get::<i64, _>("total"), total as i64);

        sink.write(slot as u64, Some(sig), &event).await.unwrap();
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM staked WHERE slot = $1",
        )
        .bind(slot)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(count, 1, "ON CONFLICT DO NOTHING must hold (staked)");
    }

    // -------- 7. UnstakeRequested --------
    {
        let slot = SLOT_BASE + 7;
        let sig = "test-sig-7";
        let amount: u64 = 25_000_000;
        let cooldown_until: i64 = 1_700_604_800;
        let event = DecodedEvent::UnstakeRequested(UnstakeRequested {
            provider_id: [0x71u8; 32],
            owner: PubkeyBytes([0x72u8; 32]),
            amount,
            cooldown_until,
        });
        sink.write(slot as u64, Some(sig), &event).await.unwrap();

        let row = sqlx::query(
            "SELECT slot, signature, provider_id, owner, amount, cooldown_until \
             FROM unstake_requested WHERE slot = $1",
        )
        .bind(slot)
        .fetch_one(&pool)
        .await
        .expect("unstake_requested row exists");
        assert_eq!(row.get::<i64, _>("slot"), slot);
        assert_eq!(row.get::<String, _>("signature"), sig);
        assert_eq!(row.get::<Vec<u8>, _>("provider_id"), vec![0x71u8; 32]);
        assert_eq!(row.get::<Vec<u8>, _>("owner"), vec![0x72u8; 32]);
        assert_eq!(row.get::<i64, _>("amount"), amount as i64);
        assert_eq!(row.get::<i64, _>("cooldown_until"), cooldown_until);

        sink.write(slot as u64, Some(sig), &event).await.unwrap();
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM unstake_requested WHERE slot = $1",
        )
        .bind(slot)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(count, 1, "ON CONFLICT DO NOTHING must hold (unstake_requested)");
    }

    // -------- 8. Unstaked --------
    {
        let slot = SLOT_BASE + 8;
        let sig = "test-sig-8";
        let amount: u64 = 25_000_000;
        let event = DecodedEvent::Unstaked(Unstaked {
            provider_id: [0x81u8; 32],
            owner: PubkeyBytes([0x82u8; 32]),
            amount,
        });
        sink.write(slot as u64, Some(sig), &event).await.unwrap();

        let row = sqlx::query(
            "SELECT slot, signature, provider_id, owner, amount \
             FROM unstaked WHERE slot = $1",
        )
        .bind(slot)
        .fetch_one(&pool)
        .await
        .expect("unstaked row exists");
        assert_eq!(row.get::<i64, _>("slot"), slot);
        assert_eq!(row.get::<String, _>("signature"), sig);
        assert_eq!(row.get::<Vec<u8>, _>("provider_id"), vec![0x81u8; 32]);
        assert_eq!(row.get::<Vec<u8>, _>("owner"), vec![0x82u8; 32]);
        assert_eq!(row.get::<i64, _>("amount"), amount as i64);

        sink.write(slot as u64, Some(sig), &event).await.unwrap();
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM unstaked WHERE slot = $1",
        )
        .bind(slot)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(count, 1, "ON CONFLICT DO NOTHING must hold (unstaked)");
    }

    // -------- 9. Slashed --------
    {
        let slot = SLOT_BASE + 9;
        let sig = "test-sig-9";
        let amount: u64 = 10_000_000;
        let event = DecodedEvent::Slashed(Slashed {
            provider_id: [0x91u8; 32],
            owner: PubkeyBytes([0x92u8; 32]),
            amount,
            destination: PubkeyBytes([0x93u8; 32]),
        });
        sink.write(slot as u64, Some(sig), &event).await.unwrap();

        let row = sqlx::query(
            "SELECT slot, signature, provider_id, owner, amount, destination \
             FROM slashed WHERE slot = $1",
        )
        .bind(slot)
        .fetch_one(&pool)
        .await
        .expect("slashed row exists");
        assert_eq!(row.get::<i64, _>("slot"), slot);
        assert_eq!(row.get::<String, _>("signature"), sig);
        assert_eq!(row.get::<Vec<u8>, _>("provider_id"), vec![0x91u8; 32]);
        assert_eq!(row.get::<Vec<u8>, _>("owner"), vec![0x92u8; 32]);
        assert_eq!(row.get::<i64, _>("amount"), amount as i64);
        assert_eq!(row.get::<Vec<u8>, _>("destination"), vec![0x93u8; 32]);

        sink.write(slot as u64, Some(sig), &event).await.unwrap();
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM slashed WHERE slot = $1",
        )
        .bind(slot)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(count, 1, "ON CONFLICT DO NOTHING must hold (slashed)");
    }

    // -------- 10. StakingInitialized --------
    {
        let slot = SLOT_BASE + 10;
        let sig = "test-sig-10";
        let min_stake: u64 = 50_000_000;
        let verified_stake: u64 = 500_000_000;
        let cooldown_secs: i64 = 7 * 86_400;
        let event = DecodedEvent::StakingInitialized(StakingInitialized {
            stake_mint: PubkeyBytes([0xa1u8; 32]),
            min_stake,
            verified_stake,
            cooldown_secs,
        });
        sink.write(slot as u64, Some(sig), &event).await.unwrap();

        let row = sqlx::query(
            "SELECT slot, signature, stake_mint, min_stake, verified_stake, cooldown_secs \
             FROM staking_initialized WHERE slot = $1",
        )
        .bind(slot)
        .fetch_one(&pool)
        .await
        .expect("staking_initialized row exists");
        assert_eq!(row.get::<i64, _>("slot"), slot);
        assert_eq!(row.get::<String, _>("signature"), sig);
        assert_eq!(row.get::<Vec<u8>, _>("stake_mint"), vec![0xa1u8; 32]);
        assert_eq!(row.get::<i64, _>("min_stake"), min_stake as i64);
        assert_eq!(row.get::<i64, _>("verified_stake"), verified_stake as i64);
        assert_eq!(row.get::<i64, _>("cooldown_secs"), cooldown_secs);

        sink.write(slot as u64, Some(sig), &event).await.unwrap();
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM staking_initialized WHERE slot = $1",
        )
        .bind(slot)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(count, 1, "ON CONFLICT DO NOTHING must hold (staking_initialized)");
    }

    // -------- 11. StakeMintSet --------
    {
        let slot = SLOT_BASE + 11;
        let sig = "test-sig-11";
        let event = DecodedEvent::StakeMintSet(StakeMintSet {
            stake_mint: PubkeyBytes([0xb1u8; 32]),
        });
        sink.write(slot as u64, Some(sig), &event).await.unwrap();

        let row = sqlx::query(
            "SELECT slot, signature, stake_mint \
             FROM stake_mint_set WHERE slot = $1",
        )
        .bind(slot)
        .fetch_one(&pool)
        .await
        .expect("stake_mint_set row exists");
        assert_eq!(row.get::<i64, _>("slot"), slot);
        assert_eq!(row.get::<String, _>("signature"), sig);
        assert_eq!(row.get::<Vec<u8>, _>("stake_mint"), vec![0xb1u8; 32]);

        sink.write(slot as u64, Some(sig), &event).await.unwrap();
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM stake_mint_set WHERE slot = $1",
        )
        .bind(slot)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(count, 1, "ON CONFLICT DO NOTHING must hold (stake_mint_set)");
    }

    // -------- 12. RoutingFeeProcessed --------
    {
        let slot = SLOT_BASE + 12;
        let sig = "test-sig-12";
        let burned: u64 = 500_000;
        let to_stakers: u64 = 1_500_000;
        let event = DecodedEvent::RoutingFeeProcessed(RoutingFeeProcessed {
            burned,
            to_stakers,
        });
        sink.write(slot as u64, Some(sig), &event).await.unwrap();

        let row = sqlx::query(
            "SELECT slot, signature, burned, to_stakers \
             FROM routing_fee_processed WHERE slot = $1",
        )
        .bind(slot)
        .fetch_one(&pool)
        .await
        .expect("routing_fee_processed row exists");
        assert_eq!(row.get::<i64, _>("slot"), slot);
        assert_eq!(row.get::<String, _>("signature"), sig);
        assert_eq!(row.get::<i64, _>("burned"), burned as i64);
        assert_eq!(row.get::<i64, _>("to_stakers"), to_stakers as i64);

        sink.write(slot as u64, Some(sig), &event).await.unwrap();
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM routing_fee_processed WHERE slot = $1",
        )
        .bind(slot)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(count, 1, "ON CONFLICT DO NOTHING must hold (routing_fee_processed)");
    }

    // -------- 13. ReputationVoteBurned --------
    {
        let slot = SLOT_BASE + 13;
        let sig = "test-sig-13";
        let event = DecodedEvent::ReputationVoteBurned(ReputationVoteBurned {
            voter: PubkeyBytes([0xc1u8; 32]),
            provider_id: [0xc2u8; 32],
        });
        sink.write(slot as u64, Some(sig), &event).await.unwrap();

        let row = sqlx::query(
            "SELECT slot, signature, voter, provider_id \
             FROM reputation_vote_burned WHERE slot = $1",
        )
        .bind(slot)
        .fetch_one(&pool)
        .await
        .expect("reputation_vote_burned row exists");
        assert_eq!(row.get::<i64, _>("slot"), slot);
        assert_eq!(row.get::<String, _>("signature"), sig);
        assert_eq!(row.get::<Vec<u8>, _>("voter"), vec![0xc1u8; 32]);
        assert_eq!(row.get::<Vec<u8>, _>("provider_id"), vec![0xc2u8; 32]);

        sink.write(slot as u64, Some(sig), &event).await.unwrap();
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM reputation_vote_burned WHERE slot = $1",
        )
        .bind(slot)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(count, 1, "ON CONFLICT DO NOTHING must hold (reputation_vote_burned)");
    }

    // -------- 15. VaultProofVerified --------
    {
        let slot = SLOT_BASE + 15;
        let sig = "test-sig-15";
        let on_chain_slot: u64 = 9_876_543;
        let event = DecodedEvent::VaultProofVerified(VaultProofVerified {
            provider_id: [0xe1u8; 32],
            tier: VaultTier::ZkAggregate,
            signals_hash: [0xe2u8; 32],
            submitter: PubkeyBytes([0xe3u8; 32]),
            slot: on_chain_slot,
        });
        sink.write(slot as u64, Some(sig), &event).await.unwrap();

        let row = sqlx::query(
            "SELECT slot, signature, provider_id, tier, signals_hash, submitter, on_chain_slot \
             FROM vault_proof_verified WHERE slot = $1",
        )
        .bind(slot)
        .fetch_one(&pool)
        .await
        .expect("vault_proof_verified row exists");
        assert_eq!(row.get::<i64, _>("slot"), slot);
        assert_eq!(row.get::<String, _>("signature"), sig);
        assert_eq!(row.get::<Vec<u8>, _>("provider_id"), vec![0xe1u8; 32]);
        assert_eq!(row.get::<i16, _>("tier"), VaultTier::ZkAggregate as i16);
        assert_eq!(row.get::<Vec<u8>, _>("signals_hash"), vec![0xe2u8; 32]);
        assert_eq!(row.get::<Vec<u8>, _>("submitter"), vec![0xe3u8; 32]);
        assert_eq!(row.get::<i64, _>("on_chain_slot"), on_chain_slot as i64);

        sink.write(slot as u64, Some(sig), &event).await.unwrap();
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM vault_proof_verified WHERE slot = $1",
        )
        .bind(slot)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(count, 1, "ON CONFLICT DO NOTHING must hold (vault_proof_verified)");
    }

    // -------- 16. ReputationScoreSet --------
    {
        let slot = SLOT_BASE + 16;
        let sig = "test-sig-16";
        let score: u16 = 825;
        let event = DecodedEvent::ReputationScoreSet(ReputationScoreSet {
            provider_id: [0xf1u8; 32],
            score,
        });
        sink.write(slot as u64, Some(sig), &event).await.unwrap();

        let row = sqlx::query(
            "SELECT slot, signature, provider_id, score \
             FROM reputation_score_set WHERE slot = $1",
        )
        .bind(slot)
        .fetch_one(&pool)
        .await
        .expect("reputation_score_set row exists");
        assert_eq!(row.get::<i64, _>("slot"), slot);
        assert_eq!(row.get::<String, _>("signature"), sig);
        assert_eq!(row.get::<Vec<u8>, _>("provider_id"), vec![0xf1u8; 32]);
        assert_eq!(row.get::<i32, _>("score"), score as i32);

        sink.write(slot as u64, Some(sig), &event).await.unwrap();
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM reputation_score_set WHERE slot = $1",
        )
        .bind(slot)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(count, 1, "ON CONFLICT DO NOTHING must hold (reputation_score_set)");
    }

    // -------- 17. EscrowInitialized --------
    {
        let slot = SLOT_BASE + 17;
        let sig = "test-sig-17";
        let event = DecodedEvent::EscrowInitialized(EscrowInitialized {
            admin: PubkeyBytes([0xa1u8; 32]),
            usdc_mint: PubkeyBytes([0xa2u8; 32]),
            router: PubkeyBytes([0xa3u8; 32]),
        });
        sink.write(slot as u64, Some(sig), &event).await.unwrap();

        let row = sqlx::query(
            "SELECT slot, signature, admin, usdc_mint, router \
             FROM escrow_initialized WHERE slot = $1",
        )
        .bind(slot)
        .fetch_one(&pool)
        .await
        .expect("escrow_initialized row exists");
        assert_eq!(row.get::<i64, _>("slot"), slot);
        assert_eq!(row.get::<String, _>("signature"), sig);
        assert_eq!(row.get::<Vec<u8>, _>("admin"), vec![0xa1u8; 32]);
        assert_eq!(row.get::<Vec<u8>, _>("usdc_mint"), vec![0xa2u8; 32]);
        assert_eq!(row.get::<Vec<u8>, _>("router"), vec![0xa3u8; 32]);

        sink.write(slot as u64, Some(sig), &event).await.unwrap();
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM escrow_initialized WHERE slot = $1",
        )
        .bind(slot)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(count, 1, "ON CONFLICT DO NOTHING must hold (escrow_initialized)");
    }

    // -------- 18. SessionOpened --------
    {
        let slot = SLOT_BASE + 18;
        let sig = "test-sig-18";
        let deposit: u64 = 5_000_000;
        let spending_limit: u64 = 2_500_000;
        let expires_at: i64 = 1_750_000_000;
        let event = DecodedEvent::SessionOpened(SessionOpened {
            session_id: [0xb1u8; 32],
            agent_wallet: PubkeyBytes([0xb2u8; 32]),
            deposit,
            spending_limit,
            expires_at,
        });
        sink.write(slot as u64, Some(sig), &event).await.unwrap();

        let row = sqlx::query(
            "SELECT slot, signature, session_id, agent_wallet, deposit, spending_limit, expires_at \
             FROM session_opened WHERE slot = $1",
        )
        .bind(slot)
        .fetch_one(&pool)
        .await
        .expect("session_opened row exists");
        assert_eq!(row.get::<i64, _>("slot"), slot);
        assert_eq!(row.get::<String, _>("signature"), sig);
        assert_eq!(row.get::<Vec<u8>, _>("session_id"), vec![0xb1u8; 32]);
        assert_eq!(row.get::<Vec<u8>, _>("agent_wallet"), vec![0xb2u8; 32]);
        assert_eq!(row.get::<i64, _>("deposit"), deposit as i64);
        assert_eq!(row.get::<i64, _>("spending_limit"), spending_limit as i64);
        assert_eq!(row.get::<i64, _>("expires_at"), expires_at);

        sink.write(slot as u64, Some(sig), &event).await.unwrap();
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM session_opened WHERE slot = $1",
        )
        .bind(slot)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(count, 1, "ON CONFLICT DO NOTHING must hold (session_opened)");
    }

    // -------- 19. VoucherSettled --------
    {
        let slot = SLOT_BASE + 19;
        let sig = "test-sig-19";
        let cumulative_amount: u64 = 1_000_000;
        let delta: u64 = 250_000;
        let to_provider: u64 = 245_000;
        let fee: u64 = 5_000;
        let nonce: u64 = 4;
        let event = DecodedEvent::VoucherSettled(VoucherSettled {
            session_id: [0xc1u8; 32],
            provider_id: [0xc2u8; 32],
            cumulative_amount,
            delta,
            to_provider,
            fee,
            nonce,
        });
        sink.write(slot as u64, Some(sig), &event).await.unwrap();

        let row = sqlx::query(
            "SELECT slot, signature, session_id, provider_id, cumulative_amount, delta, to_provider, fee, nonce \
             FROM voucher_settled WHERE slot = $1",
        )
        .bind(slot)
        .fetch_one(&pool)
        .await
        .expect("voucher_settled row exists");
        assert_eq!(row.get::<i64, _>("slot"), slot);
        assert_eq!(row.get::<String, _>("signature"), sig);
        assert_eq!(row.get::<Vec<u8>, _>("session_id"), vec![0xc1u8; 32]);
        assert_eq!(row.get::<Vec<u8>, _>("provider_id"), vec![0xc2u8; 32]);
        assert_eq!(row.get::<i64, _>("cumulative_amount"), cumulative_amount as i64);
        assert_eq!(row.get::<i64, _>("delta"), delta as i64);
        assert_eq!(row.get::<i64, _>("to_provider"), to_provider as i64);
        assert_eq!(row.get::<i64, _>("fee"), fee as i64);
        assert_eq!(row.get::<i64, _>("nonce"), nonce as i64);

        sink.write(slot as u64, Some(sig), &event).await.unwrap();
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM voucher_settled WHERE slot = $1",
        )
        .bind(slot)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(count, 1, "ON CONFLICT DO NOTHING must hold (voucher_settled)");
    }

    // -------- 20. SessionSettled --------
    {
        let slot = SLOT_BASE + 20;
        let sig = "test-sig-20";
        let total_to_providers: u64 = 1_000_000;
        let routing_fee: u64 = 20_000;
        let refund_to_agent: u64 = 480_000;
        let event = DecodedEvent::SessionSettled(SessionSettled {
            session_id: [0xd3u8; 32],
            total_to_providers,
            routing_fee,
            refund_to_agent,
        });
        sink.write(slot as u64, Some(sig), &event).await.unwrap();

        let row = sqlx::query(
            "SELECT slot, signature, session_id, total_to_providers, routing_fee, refund_to_agent \
             FROM session_settled WHERE slot = $1",
        )
        .bind(slot)
        .fetch_one(&pool)
        .await
        .expect("session_settled row exists");
        assert_eq!(row.get::<i64, _>("slot"), slot);
        assert_eq!(row.get::<String, _>("signature"), sig);
        assert_eq!(row.get::<Vec<u8>, _>("session_id"), vec![0xd3u8; 32]);
        assert_eq!(row.get::<i64, _>("total_to_providers"), total_to_providers as i64);
        assert_eq!(row.get::<i64, _>("routing_fee"), routing_fee as i64);
        assert_eq!(row.get::<i64, _>("refund_to_agent"), refund_to_agent as i64);

        sink.write(slot as u64, Some(sig), &event).await.unwrap();
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM session_settled WHERE slot = $1",
        )
        .bind(slot)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(count, 1, "ON CONFLICT DO NOTHING must hold (session_settled)");
    }

    // -------- 21. VaultConfigured --------
    {
        let slot = SLOT_BASE + 21;
        let sig = "test-sig-21";
        let event = DecodedEvent::VaultConfigured(VaultConfigured {
            provider_id: [0xe4u8; 32],
            tier: VaultTier::Confidential,
            on_chain_verifier: PubkeyBytes([0xe5u8; 32]),
            arweave_cid: [0xe6u8; 32],
        });
        sink.write(slot as u64, Some(sig), &event).await.unwrap();

        let row = sqlx::query(
            "SELECT slot, signature, provider_id, tier, on_chain_verifier, arweave_cid \
             FROM vault_configured WHERE slot = $1",
        )
        .bind(slot)
        .fetch_one(&pool)
        .await
        .expect("vault_configured row exists");
        assert_eq!(row.get::<i64, _>("slot"), slot);
        assert_eq!(row.get::<String, _>("signature"), sig);
        assert_eq!(row.get::<Vec<u8>, _>("provider_id"), vec![0xe4u8; 32]);
        assert_eq!(row.get::<i16, _>("tier"), VaultTier::Confidential as i16);
        assert_eq!(row.get::<Vec<u8>, _>("on_chain_verifier"), vec![0xe5u8; 32]);
        assert_eq!(row.get::<Vec<u8>, _>("arweave_cid"), vec![0xe6u8; 32]);

        sink.write(slot as u64, Some(sig), &event).await.unwrap();
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM vault_configured WHERE slot = $1",
        )
        .bind(slot)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(count, 1, "ON CONFLICT DO NOTHING must hold (vault_configured)");
    }

    // -------- 22. VaultAuditorKeySet --------
    {
        let slot = SLOT_BASE + 22;
        let sig = "test-sig-22";
        let event = DecodedEvent::VaultAuditorKeySet(VaultAuditorKeySet {
            provider_id: [0xf3u8; 32],
            previous: PubkeyBytes([0xf4u8; 32]),
            current: PubkeyBytes([0xf5u8; 32]),
        });
        sink.write(slot as u64, Some(sig), &event).await.unwrap();

        let row = sqlx::query(
            "SELECT slot, signature, provider_id, previous, current_key \
             FROM vault_auditor_key_set WHERE slot = $1",
        )
        .bind(slot)
        .fetch_one(&pool)
        .await
        .expect("vault_auditor_key_set row exists");
        assert_eq!(row.get::<i64, _>("slot"), slot);
        assert_eq!(row.get::<String, _>("signature"), sig);
        assert_eq!(row.get::<Vec<u8>, _>("provider_id"), vec![0xf3u8; 32]);
        assert_eq!(row.get::<Vec<u8>, _>("previous"), vec![0xf4u8; 32]);
        assert_eq!(row.get::<Vec<u8>, _>("current_key"), vec![0xf5u8; 32]);

        sink.write(slot as u64, Some(sig), &event).await.unwrap();
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM vault_auditor_key_set WHERE slot = $1",
        )
        .bind(slot)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(count, 1, "ON CONFLICT DO NOTHING must hold (vault_auditor_key_set)");
    }

    // -------- 23. VaultBuyerKeyRotationUpdated --------
    {
        let slot = SLOT_BASE + 23;
        let sig = "test-sig-23";
        let event = DecodedEvent::VaultBuyerKeyRotationUpdated(VaultBuyerKeyRotationUpdated {
            provider_id: [0xa5u8; 32],
            cid: [0xa6u8; 32],
        });
        sink.write(slot as u64, Some(sig), &event).await.unwrap();

        let row = sqlx::query(
            "SELECT slot, signature, provider_id, cid \
             FROM vault_buyer_key_rotation_updated WHERE slot = $1",
        )
        .bind(slot)
        .fetch_one(&pool)
        .await
        .expect("vault_buyer_key_rotation_updated row exists");
        assert_eq!(row.get::<i64, _>("slot"), slot);
        assert_eq!(row.get::<String, _>("signature"), sig);
        assert_eq!(row.get::<Vec<u8>, _>("provider_id"), vec![0xa5u8; 32]);
        assert_eq!(row.get::<Vec<u8>, _>("cid"), vec![0xa6u8; 32]);

        sink.write(slot as u64, Some(sig), &event).await.unwrap();
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM vault_buyer_key_rotation_updated WHERE slot = $1",
        )
        .bind(slot)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(
            count, 1,
            "ON CONFLICT DO NOTHING must hold (vault_buyer_key_rotation_updated)"
        );
    }

    // -------- 24. VaultDeactivated --------
    {
        let slot = SLOT_BASE + 24;
        let sig = "test-sig-24";
        let event = DecodedEvent::VaultDeactivated(VaultDeactivated {
            provider_id: [0xb7u8; 32],
        });
        sink.write(slot as u64, Some(sig), &event).await.unwrap();

        let row = sqlx::query(
            "SELECT slot, signature, provider_id FROM vault_deactivated WHERE slot = $1",
        )
        .bind(slot)
        .fetch_one(&pool)
        .await
        .expect("vault_deactivated row exists");
        assert_eq!(row.get::<i64, _>("slot"), slot);
        assert_eq!(row.get::<String, _>("signature"), sig);
        assert_eq!(row.get::<Vec<u8>, _>("provider_id"), vec![0xb7u8; 32]);

        sink.write(slot as u64, Some(sig), &event).await.unwrap();
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM vault_deactivated WHERE slot = $1",
        )
        .bind(slot)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(count, 1, "ON CONFLICT DO NOTHING must hold (vault_deactivated)");
    }
}
