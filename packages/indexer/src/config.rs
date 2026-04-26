use std::env;

use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub struct Config {
    pub network: String,
    pub program_id: String,
    pub yellowstone_url: Option<String>,
    pub database_url: Option<String>,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let network = env::var("STARGAZE_NETWORK").unwrap_or_else(|_| "solana-devnet".to_string());
        let program_id = env::var("STARGAZE_ANCHOR_PROGRAM_ID")
            .context("STARGAZE_ANCHOR_PROGRAM_ID must be set (publish from packages/shared/src/solana/programs.ts)")?;
        let yellowstone_url = env::var("YELLOWSTONE_GRPC_URL").ok();
        let database_url = env::var("DATABASE_URL").ok();
        Ok(Config { network, program_id, yellowstone_url, database_url })
    }
}
