use std::collections::HashMap;

use anyhow::{Context, Result};
use futures::{SinkExt, StreamExt};
use tokio::{select, signal};
use tracing::{debug, error, info, warn};
use yellowstone_grpc_client::{ClientTlsConfig, GeyserGrpcClient};
use yellowstone_grpc_proto::geyser::{
    subscribe_update::UpdateOneof, CommitmentLevel, SubscribeRequest,
    SubscribeRequestFilterTransactions, SubscribeUpdate,
};

use crate::config::Config;

/// Connects to a Yellowstone gRPC endpoint and subscribes to every
/// transaction that touches the configured `StargazeAnchor` program.
/// Emits a structured JSON event per inbound transaction; downstream
/// projection into Postgres + TimescaleDB lands once the shared Drizzle
/// schema is published.
pub async fn run(cfg: Config) -> Result<()> {
    let Some(yellowstone_url) = cfg.yellowstone_url.clone() else {
        warn!("YELLOWSTONE_GRPC_URL not set — running in dry mode");
        signal::ctrl_c().await?;
        return Ok(());
    };

    info!(
        program_id = %cfg.program_id,
        url = %yellowstone_url,
        has_db = cfg.database_url.is_some(),
        "subscribing to StargazeAnchor program activity",
    );

    let mut client = GeyserGrpcClient::build_from_shared(yellowstone_url)
        .context("yellowstone: invalid endpoint")?
        .x_token::<String>(None)
        .context("yellowstone: x-token config")?
        .tls_config(ClientTlsConfig::new().with_native_roots())
        .context("yellowstone: tls config")?
        .connect()
        .await
        .context("yellowstone: connect")?;

    let (mut subscribe_tx, mut stream) = client
        .subscribe()
        .await
        .context("yellowstone: subscribe handshake")?;

    let request = build_subscribe_request(&cfg.program_id);
    subscribe_tx
        .send(request)
        .await
        .context("yellowstone: send subscription filter")?;

    info!("yellowstone subscription open — waiting on events…");

    loop {
        select! {
            biased;
            _ = signal::ctrl_c() => {
                info!("ctrl-c received — closing yellowstone stream");
                break;
            }
            maybe = stream.next() => {
                let Some(message) = maybe else {
                    warn!("yellowstone stream ended — exiting");
                    break;
                };
                match message {
                    Ok(update) => handle_update(update),
                    Err(err) => error!(error = %err, "yellowstone stream error"),
                }
            }
        }
    }

    Ok(())
}

fn build_subscribe_request(program_id: &str) -> SubscribeRequest {
    let mut transactions = HashMap::new();
    transactions.insert(
        "stargaze_anchor".to_string(),
        SubscribeRequestFilterTransactions {
            vote: Some(false),
            failed: Some(false),
            signature: None,
            account_include: vec![program_id.to_string()],
            account_exclude: vec![],
            account_required: vec![],
        },
    );

    SubscribeRequest {
        accounts: HashMap::default(),
        slots: HashMap::default(),
        transactions,
        transactions_status: HashMap::default(),
        blocks: HashMap::default(),
        blocks_meta: HashMap::default(),
        entry: HashMap::default(),
        commitment: Some(CommitmentLevel::Confirmed as i32),
        accounts_data_slice: vec![],
        ping: None,
        from_slot: None,
    }
}

fn handle_update(update: SubscribeUpdate) {
    let Some(payload) = update.update_oneof else { return };
    match payload {
        UpdateOneof::Transaction(tx) => {
            let slot = tx.slot;
            let signature = tx
                .transaction
                .as_ref()
                .map(|t| hex::encode(&t.signature));
            info!(
                slot,
                signature = signature.as_deref(),
                "stargaze_anchor tx observed",
            );
        }
        UpdateOneof::Ping(_) | UpdateOneof::Pong(_) => debug!("yellowstone heartbeat"),
        _ => debug!("yellowstone update (other)"),
    }
}
