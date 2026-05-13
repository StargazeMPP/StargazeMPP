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
    CcipDispatched, DecodedEvent, ProviderRegistered, PubkeyBytes, ReputationMirrored,
    ReputationVoteBurned, ReputationVoted, RoutingFeeProcessed, Slashed, StakeDispatched,
    StakeMintSet, Staked, StakingInitialized, UnstakeRequested, Unstaked, X402ReceiptRecorded,
};
use stargaze_indexer::sink::{EventSink, PostgresSink};

/// All 14 projection tables — the pre-clean step deletes the test slot
/// range from every one of them before the fixtures are inserted.
const ALL_TABLES: &[&str] = &[
    "provider_registered",
    "reputation_voted",
    "x402_receipt_recorded",
    "reputation_mirrored",
    "ccip_dispatched",
    "staked",
    "unstake_requested",
    "unstaked",
    "slashed",
    "staking_initialized",
    "stake_mint_set",
    "routing_fee_processed",
    "reputation_vote_burned",
    "stake_dispatched",
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

    // -------- 4. ReputationMirrored --------
    {
        let slot = SLOT_BASE + 4;
        let sig = "test-sig-4";
        let score: u16 = 750;
        let event = DecodedEvent::ReputationMirrored(ReputationMirrored {
            provider_id: [0x41u8; 32],
            score,
        });
        sink.write(slot as u64, Some(sig), &event).await.unwrap();

        let row = sqlx::query(
            "SELECT slot, signature, provider_id, score \
             FROM reputation_mirrored WHERE slot = $1",
        )
        .bind(slot)
        .fetch_one(&pool)
        .await
        .expect("reputation_mirrored row exists");
        assert_eq!(row.get::<i64, _>("slot"), slot);
        assert_eq!(row.get::<String, _>("signature"), sig);
        assert_eq!(row.get::<Vec<u8>, _>("provider_id"), vec![0x41u8; 32]);
        assert_eq!(row.get::<i32, _>("score"), score as i32);

        sink.write(slot as u64, Some(sig), &event).await.unwrap();
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM reputation_mirrored WHERE slot = $1",
        )
        .bind(slot)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(count, 1, "ON CONFLICT DO NOTHING must hold (reputation_mirrored)");
    }

    // -------- 5. CcipDispatched --------
    {
        let slot = SLOT_BASE + 5;
        let sig = "test-sig-5";
        let score: u16 = 875;
        let dest: u64 = 16_015_286_601_757_825_753;
        let receiver = vec![0xde, 0xad, 0xbe, 0xef];
        let payload = vec![1, 2, 3, 4, 5];
        let extra_args = vec![0xaa, 0xbb];
        let event = DecodedEvent::CcipDispatched(CcipDispatched {
            provider_id: [0x51u8; 32],
            score,
            dest_chain_selector: dest,
            receiver: receiver.clone(),
            payload: payload.clone(),
            extra_args: extra_args.clone(),
        });
        sink.write(slot as u64, Some(sig), &event).await.unwrap();

        let row = sqlx::query(
            "SELECT slot, signature, provider_id, score, dest_chain_selector, receiver, payload, extra_args \
             FROM ccip_dispatched WHERE slot = $1",
        )
        .bind(slot)
        .fetch_one(&pool)
        .await
        .expect("ccip_dispatched row exists");
        assert_eq!(row.get::<i64, _>("slot"), slot);
        assert_eq!(row.get::<String, _>("signature"), sig);
        assert_eq!(row.get::<Vec<u8>, _>("provider_id"), vec![0x51u8; 32]);
        assert_eq!(row.get::<i32, _>("score"), score as i32);
        assert_eq!(row.get::<i64, _>("dest_chain_selector"), dest as i64);
        assert_eq!(row.get::<Vec<u8>, _>("receiver"), receiver);
        assert_eq!(row.get::<Vec<u8>, _>("payload"), payload);
        assert_eq!(row.get::<Vec<u8>, _>("extra_args"), extra_args);

        sink.write(slot as u64, Some(sig), &event).await.unwrap();
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM ccip_dispatched WHERE slot = $1",
        )
        .bind(slot)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(count, 1, "ON CONFLICT DO NOTHING must hold (ccip_dispatched)");
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

    // -------- 14. StakeDispatched --------
    {
        let slot = SLOT_BASE + 14;
        let sig = "test-sig-14";
        let amount: u64 = 100_000_000;
        let dest: u64 = 16_015_286_601_757_825_753;
        let receiver = vec![0xfe, 0xed, 0xfa, 0xce];
        let payload = vec![9, 8, 7, 6, 5, 4, 3, 2, 1];
        let extra_args = vec![0xcc, 0xdd];
        let event = DecodedEvent::StakeDispatched(StakeDispatched {
            provider_id: [0xd1u8; 32],
            owner: PubkeyBytes([0xd2u8; 32]),
            amount,
            dest_chain_selector: dest,
            receiver: receiver.clone(),
            payload: payload.clone(),
            extra_args: extra_args.clone(),
        });
        sink.write(slot as u64, Some(sig), &event).await.unwrap();

        let row = sqlx::query(
            "SELECT slot, signature, provider_id, owner, amount, dest_chain_selector, receiver, payload, extra_args \
             FROM stake_dispatched WHERE slot = $1",
        )
        .bind(slot)
        .fetch_one(&pool)
        .await
        .expect("stake_dispatched row exists");
        assert_eq!(row.get::<i64, _>("slot"), slot);
        assert_eq!(row.get::<String, _>("signature"), sig);
        assert_eq!(row.get::<Vec<u8>, _>("provider_id"), vec![0xd1u8; 32]);
        assert_eq!(row.get::<Vec<u8>, _>("owner"), vec![0xd2u8; 32]);
        assert_eq!(row.get::<i64, _>("amount"), amount as i64);
        assert_eq!(row.get::<i64, _>("dest_chain_selector"), dest as i64);
        assert_eq!(row.get::<Vec<u8>, _>("receiver"), receiver);
        assert_eq!(row.get::<Vec<u8>, _>("payload"), payload);
        assert_eq!(row.get::<Vec<u8>, _>("extra_args"), extra_args);

        sink.write(slot as u64, Some(sig), &event).await.unwrap();
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM stake_dispatched WHERE slot = $1",
        )
        .bind(slot)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(count, 1, "ON CONFLICT DO NOTHING must hold (stake_dispatched)");
    }
}
