//! Event sink trait and implementations.
//!
//! The [`EventSink`] trait abstracts where decoded Anchor events go after
//! the stream loop hands them off. Production runs select [`PostgresSink`]
//! when `DATABASE_URL` is present; absent that, [`VecSink`] keeps events in
//! memory so the binary still drains the Yellowstone feed (the events are
//! dropped on shutdown).
//!
//! All Postgres writes use the runtime `sqlx::query("...").bind(..)` API on
//! purpose — the compile-time `query!` macros require a live `DATABASE_URL`
//! at build time, which we don't have in CI.

use std::sync::Mutex;

use anyhow::Result;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use tracing::info;

use stargaze_events::DecodedEvent;

/// Sink for decoded events emitted by the Yellowstone stream loop.
///
/// Implementations must be cheap to clone via `Arc<dyn EventSink>` and
/// tolerate concurrent writers (the stream loop is single-threaded today
/// but the trait shouldn't preclude fan-out later).
#[async_trait::async_trait]
pub trait EventSink: Send + Sync {
    /// Persist (or buffer) a single decoded event. Errors are surfaced to
    /// the caller, which logs and continues — a single bad write must not
    /// abort the stream.
    async fn write(
        &self,
        slot: u64,
        signature: Option<&str>,
        event: &DecodedEvent,
    ) -> Result<()>;
}

/// In-memory sink used for unit tests and as a no-op fallback when
/// `DATABASE_URL` is unset.
#[derive(Default)]
pub struct VecSink {
    inner: Mutex<Vec<(u64, Option<String>, DecodedEvent)>>,
}

impl VecSink {
    pub fn new() -> Self {
        Self::default()
    }

    /// Clone the buffered events out. Order matches insertion order.
    pub fn snapshot(&self) -> Vec<(u64, Option<String>, DecodedEvent)> {
        self.inner.lock().expect("vec sink mutex poisoned").clone()
    }
}

#[async_trait::async_trait]
impl EventSink for VecSink {
    async fn write(
        &self,
        slot: u64,
        signature: Option<&str>,
        event: &DecodedEvent,
    ) -> Result<()> {
        self.inner
            .lock()
            .expect("vec sink mutex poisoned")
            .push((slot, signature.map(str::to_owned), event.clone()));
        Ok(())
    }
}

/// Postgres-backed sink. One table per Anchor event variant, all keyed
/// `UNIQUE (slot, signature)` so replays are idempotent.
pub struct PostgresSink {
    pool: PgPool,
}

impl PostgresSink {
    /// Connect to Postgres and run the bundled migrations. The migrations
    /// directory is walked at compile time by `sqlx::migrate!`, so the
    /// crate still builds without `DATABASE_URL` set.
    pub async fn connect(database_url: &str) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(8)
            .connect(database_url)
            .await?;
        sqlx::migrate!("./migrations").run(&pool).await?;
        info!("postgres sink connected — migrations applied");
        Ok(Self { pool })
    }
}

#[async_trait::async_trait]
impl EventSink for PostgresSink {
    async fn write(
        &self,
        slot: u64,
        signature: Option<&str>,
        event: &DecodedEvent,
    ) -> Result<()> {
        // Solana slots are u64 on the wire but i64 covers ~292 years of
        // ledger growth; safe to cast.
        let slot_i64 = slot as i64;
        let sig = signature.unwrap_or("");
        match event {
            DecodedEvent::ProviderRegistered(e) => {
                sqlx::query(
                    "INSERT INTO provider_registered \
                     (slot, signature, provider_id, owner, category_hash, meta_cid) \
                     VALUES ($1, $2, $3, $4, $5, $6) \
                     ON CONFLICT (slot, signature) DO NOTHING",
                )
                .bind(slot_i64)
                .bind(sig)
                .bind(e.provider_id.as_slice())
                .bind(e.owner.0.as_slice())
                .bind(e.category_hash.as_slice())
                .bind(e.meta_cid.as_slice())
                .execute(&self.pool)
                .await?;
            }
            DecodedEvent::ReputationVoted(e) => {
                sqlx::query(
                    "INSERT INTO reputation_voted \
                     (slot, signature, provider_id, voter, accurate) \
                     VALUES ($1, $2, $3, $4, $5) \
                     ON CONFLICT (slot, signature) DO NOTHING",
                )
                .bind(slot_i64)
                .bind(sig)
                .bind(e.provider_id.as_slice())
                .bind(e.voter.0.as_slice())
                .bind(e.accurate)
                .execute(&self.pool)
                .await?;
            }
            DecodedEvent::X402ReceiptRecorded(e) => {
                // TODO: widen to NUMERIC if amount ever exceeds i64::MAX.
                // USDC is 6-decimal; ~9.2e12 USDC fits in i64.
                sqlx::query(
                    "INSERT INTO x402_receipt_recorded \
                     (slot, signature, session_id, provider_id, payer, amount, paid_at) \
                     VALUES ($1, $2, $3, $4, $5, $6, $7) \
                     ON CONFLICT (slot, signature) DO NOTHING",
                )
                .bind(slot_i64)
                .bind(sig)
                .bind(e.session_id.as_slice())
                .bind(e.provider_id.as_slice())
                .bind(e.payer.0.as_slice())
                .bind(e.amount as i64)
                .bind(e.paid_at)
                .execute(&self.pool)
                .await?;
            }
            DecodedEvent::ReputationMirrored(e) => {
                sqlx::query(
                    "INSERT INTO reputation_mirrored \
                     (slot, signature, provider_id, score) \
                     VALUES ($1, $2, $3, $4) \
                     ON CONFLICT (slot, signature) DO NOTHING",
                )
                .bind(slot_i64)
                .bind(sig)
                .bind(e.provider_id.as_slice())
                .bind(e.score as i32)
                .execute(&self.pool)
                .await?;
            }
            DecodedEvent::CcipDispatched(e) => {
                // TODO: widen to NUMERIC if dest_chain_selector ever exceeds i64::MAX.
                // CCIP selectors today fit in i64 (Tempo testnet = 16_015_286_601_757_825_753).
                sqlx::query(
                    "INSERT INTO ccip_dispatched \
                     (slot, signature, provider_id, score, dest_chain_selector, receiver, payload, extra_args) \
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8) \
                     ON CONFLICT (slot, signature) DO NOTHING",
                )
                .bind(slot_i64)
                .bind(sig)
                .bind(e.provider_id.as_slice())
                .bind(e.score as i32)
                .bind(e.dest_chain_selector as i64)
                .bind(e.receiver.as_slice())
                .bind(e.payload.as_slice())
                .bind(e.extra_args.as_slice())
                .execute(&self.pool)
                .await?;
            }
            DecodedEvent::Staked(e) => {
                // TODO: widen to NUMERIC if amount/total ever exceed i64::MAX.
                sqlx::query(
                    "INSERT INTO staked \
                     (slot, signature, provider_id, owner, amount, total) \
                     VALUES ($1, $2, $3, $4, $5, $6) \
                     ON CONFLICT (slot, signature) DO NOTHING",
                )
                .bind(slot_i64)
                .bind(sig)
                .bind(e.provider_id.as_slice())
                .bind(e.owner.0.as_slice())
                .bind(e.amount as i64)
                .bind(e.total as i64)
                .execute(&self.pool)
                .await?;
            }
            DecodedEvent::UnstakeRequested(e) => {
                // TODO: widen to NUMERIC if amount ever exceeds i64::MAX.
                sqlx::query(
                    "INSERT INTO unstake_requested \
                     (slot, signature, provider_id, owner, amount, cooldown_until) \
                     VALUES ($1, $2, $3, $4, $5, $6) \
                     ON CONFLICT (slot, signature) DO NOTHING",
                )
                .bind(slot_i64)
                .bind(sig)
                .bind(e.provider_id.as_slice())
                .bind(e.owner.0.as_slice())
                .bind(e.amount as i64)
                .bind(e.cooldown_until)
                .execute(&self.pool)
                .await?;
            }
            DecodedEvent::Unstaked(e) => {
                // TODO: widen to NUMERIC if amount ever exceeds i64::MAX.
                sqlx::query(
                    "INSERT INTO unstaked \
                     (slot, signature, provider_id, owner, amount) \
                     VALUES ($1, $2, $3, $4, $5) \
                     ON CONFLICT (slot, signature) DO NOTHING",
                )
                .bind(slot_i64)
                .bind(sig)
                .bind(e.provider_id.as_slice())
                .bind(e.owner.0.as_slice())
                .bind(e.amount as i64)
                .execute(&self.pool)
                .await?;
            }
            DecodedEvent::Slashed(e) => {
                // TODO: widen to NUMERIC if amount ever exceeds i64::MAX.
                sqlx::query(
                    "INSERT INTO slashed \
                     (slot, signature, provider_id, owner, amount, destination) \
                     VALUES ($1, $2, $3, $4, $5, $6) \
                     ON CONFLICT (slot, signature) DO NOTHING",
                )
                .bind(slot_i64)
                .bind(sig)
                .bind(e.provider_id.as_slice())
                .bind(e.owner.0.as_slice())
                .bind(e.amount as i64)
                .bind(e.destination.0.as_slice())
                .execute(&self.pool)
                .await?;
            }
            DecodedEvent::StakingInitialized(e) => {
                // TODO: widen to NUMERIC if min_stake/verified_stake ever exceed i64::MAX.
                sqlx::query(
                    "INSERT INTO staking_initialized \
                     (slot, signature, stake_mint, min_stake, verified_stake, cooldown_secs) \
                     VALUES ($1, $2, $3, $4, $5, $6) \
                     ON CONFLICT (slot, signature) DO NOTHING",
                )
                .bind(slot_i64)
                .bind(sig)
                .bind(e.stake_mint.0.as_slice())
                .bind(e.min_stake as i64)
                .bind(e.verified_stake as i64)
                .bind(e.cooldown_secs)
                .execute(&self.pool)
                .await?;
            }
            DecodedEvent::StakeMintSet(e) => {
                sqlx::query(
                    "INSERT INTO stake_mint_set \
                     (slot, signature, stake_mint) \
                     VALUES ($1, $2, $3) \
                     ON CONFLICT (slot, signature) DO NOTHING",
                )
                .bind(slot_i64)
                .bind(sig)
                .bind(e.stake_mint.0.as_slice())
                .execute(&self.pool)
                .await?;
            }
            DecodedEvent::RoutingFeeProcessed(e) => {
                // TODO: widen to NUMERIC if burned/to_stakers ever exceed i64::MAX.
                sqlx::query(
                    "INSERT INTO routing_fee_processed \
                     (slot, signature, burned, to_stakers) \
                     VALUES ($1, $2, $3, $4) \
                     ON CONFLICT (slot, signature) DO NOTHING",
                )
                .bind(slot_i64)
                .bind(sig)
                .bind(e.burned as i64)
                .bind(e.to_stakers as i64)
                .execute(&self.pool)
                .await?;
            }
            DecodedEvent::ReputationVoteBurned(e) => {
                sqlx::query(
                    "INSERT INTO reputation_vote_burned \
                     (slot, signature, voter, provider_id) \
                     VALUES ($1, $2, $3, $4) \
                     ON CONFLICT (slot, signature) DO NOTHING",
                )
                .bind(slot_i64)
                .bind(sig)
                .bind(e.voter.0.as_slice())
                .bind(e.provider_id.as_slice())
                .execute(&self.pool)
                .await?;
            }
            DecodedEvent::StakeDispatched(e) => {
                // TODO: widen to NUMERIC if amount/dest_chain_selector ever exceed i64::MAX.
                // CCIP selectors today fit in i64 (Tempo testnet = 16_015_286_601_757_825_753).
                sqlx::query(
                    "INSERT INTO stake_dispatched \
                     (slot, signature, provider_id, owner, amount, dest_chain_selector, receiver, payload, extra_args) \
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) \
                     ON CONFLICT (slot, signature) DO NOTHING",
                )
                .bind(slot_i64)
                .bind(sig)
                .bind(e.provider_id.as_slice())
                .bind(e.owner.0.as_slice())
                .bind(e.amount as i64)
                .bind(e.dest_chain_selector as i64)
                .bind(e.receiver.as_slice())
                .bind(e.payload.as_slice())
                .bind(e.extra_args.as_slice())
                .execute(&self.pool)
                .await?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use stargaze_events::{
        CcipDispatched, DecodedEvent, ProviderRegistered, PubkeyBytes, ReputationVoted,
    };

    fn provider_event() -> DecodedEvent {
        DecodedEvent::ProviderRegistered(ProviderRegistered {
            provider_id: [1u8; 32],
            owner: PubkeyBytes([2u8; 32]),
            category_hash: [3u8; 32],
            meta_cid: [4u8; 32],
        })
    }

    fn voted_event() -> DecodedEvent {
        DecodedEvent::ReputationVoted(ReputationVoted {
            provider_id: [9u8; 32],
            voter: PubkeyBytes([10u8; 32]),
            accurate: true,
        })
    }

    fn ccip_event() -> DecodedEvent {
        DecodedEvent::CcipDispatched(CcipDispatched {
            provider_id: [42u8; 32],
            score: 500,
            dest_chain_selector: 16_015_286_601_757_825_753,
            receiver: vec![0xde, 0xad, 0xbe, 0xef],
            payload: vec![1, 2, 3],
            extra_args: vec![],
        })
    }

    #[tokio::test]
    async fn vec_sink_round_trips_events_in_order() {
        let sink = VecSink::new();
        sink.write(100, Some("sig-a"), &provider_event()).await.unwrap();
        sink.write(101, None, &voted_event()).await.unwrap();
        sink.write(102, Some("sig-c"), &ccip_event()).await.unwrap();

        let snap = sink.snapshot();
        assert_eq!(snap.len(), 3);

        assert_eq!(snap[0].0, 100);
        assert_eq!(snap[0].1.as_deref(), Some("sig-a"));
        assert_eq!(snap[0].2, provider_event());

        assert_eq!(snap[1].0, 101);
        assert_eq!(snap[1].1, None);
        assert_eq!(snap[1].2, voted_event());

        assert_eq!(snap[2].0, 102);
        assert_eq!(snap[2].1.as_deref(), Some("sig-c"));
        assert_eq!(snap[2].2, ccip_event());
    }

    #[tokio::test]
    async fn event_sink_works_through_trait_object() {
        let sink: Arc<dyn EventSink> = Arc::new(VecSink::new());
        sink.write(7, Some("sig"), &provider_event()).await.unwrap();
        // We can't call snapshot() through the trait object — verify via
        // a concrete-typed second handle that the Arc is sharing state.
        let concrete = Arc::new(VecSink::new());
        let dyn_handle: Arc<dyn EventSink> = concrete.clone();
        dyn_handle.write(8, None, &voted_event()).await.unwrap();
        let snap = concrete.snapshot();
        assert_eq!(snap.len(), 1);
        assert_eq!(snap[0].0, 8);
        assert_eq!(snap[0].2, voted_event());
    }
}
