use anyhow::Result;
use tokio::signal;
use tracing::{info, warn};

use crate::config::Config;

/// Connect to the configured Yellowstone gRPC endpoint and stream
/// `StargazeAnchor` program logs into the Postgres projection.
///
/// Stubbed for now — the actual gRPC client is wired once
/// `packages/shared/db/schema.ts` lands so the projection has a target.
pub async fn run(cfg: Config) -> Result<()> {
    info!(
        program_id = %cfg.program_id,
        has_grpc = cfg.yellowstone_url.is_some(),
        has_db = cfg.database_url.is_some(),
        "indexer ready (stub) — awaiting yellowstone-grpc-client + shared/db schema",
    );

    if cfg.database_url.is_none() {
        warn!("DATABASE_URL not set — projections will be no-ops once stream is wired");
    }

    signal::ctrl_c().await?;
    info!("ctrl-c received — shutting down");
    Ok(())
}
