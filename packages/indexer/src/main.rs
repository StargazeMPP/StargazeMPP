use std::env;

use anyhow::Result;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenvy::dotenv();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with_target(true)
        .json()
        .init();

    let cfg = stargaze_indexer::config::Config::from_env()?;
    info!(network = %cfg.network, program_id = %cfg.program_id, "stargaze-indexer starting");

    if let Some(yellowstone_url) = env::var("YELLOWSTONE_GRPC_URL").ok() {
        info!(url = %yellowstone_url, "yellowstone gRPC endpoint configured");
    } else {
        warn!("YELLOWSTONE_GRPC_URL not set — running in dry mode (no stream)");
    }

    stargaze_indexer::stream::run(cfg).await
}
