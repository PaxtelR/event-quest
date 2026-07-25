//! EventQuest chain indexer worker — spec §16. Polls the deployed program
//! for new signatures, decodes `AttendanceRecorded` events, and upserts
//! `attendances` (plus `event_participants` points/counters and
//! `checkin_attempts.status = 'confirmed'`) — the actual source of truth
//! for a check-in's final confirmation (see
//! apps/api/src/checkins/mod.rs's header comment: `apps/api` only ever
//! gets an attempt as far as `TRANSACTION_SUBMITTED`).

mod events;
mod process;
mod rpc;

#[cfg(test)]
mod devnet_tests;

use std::time::Duration;

use eventquest_config::AppConfig;
use sqlx::postgres::PgPoolOptions;

use process::Cursor;
use rpc::RpcClient;

/// Fast-path tick: `confirmed` commitment, a small recent page — spec
/// §16's suggestion for "UI rápida".
const FAST_TICK: Duration = Duration::from_secs(5);
const FAST_TICK_PAGE_SIZE: usize = 200;

/// Reconciliation pass: `finalized` commitment (never reverts — the
/// practical mitigation for reorgs) over a wider page, run far less often
/// — spec §16's "reconciliação periódica" and "fechamento e relatórios
/// finais".
const RECONCILE_EVERY: u32 = 24; // ~every 2 minutes at a 5s fast tick
const RECONCILE_PAGE_SIZE: usize = 1000;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .json()
        .init();

    let config = AppConfig::from_env().inspect_err(|error| {
        tracing::error!(%error, "failed to load configuration");
    })?;
    tracing::info!(network = %config.solana.network, "starting eventquest-indexer");

    let db = PgPoolOptions::new()
        .max_connections(5)
        .connect(&config.database_url)
        .await
        .inspect_err(|error| tracing::error!(%error, "failed to connect to Postgres"))?;

    let redis_client = redis::Client::open(config.redis_url.clone())
        .inspect_err(|error| tracing::error!(%error, "invalid REDIS_URL"))?;
    let mut redis = redis::aio::ConnectionManager::new(redis_client)
        .await
        .inspect_err(|error| tracing::error!(%error, "failed to connect to Redis"))?;

    let rpc = RpcClient::new(config.solana.rpc_http_url.clone());
    let cursor = Cursor {
        network: config.solana.network.clone(),
        program_id: config.solana.program_id.to_string(),
    };

    let mut ticks = 0u32;
    let mut interval = tokio::time::interval(FAST_TICK);
    loop {
        interval.tick().await;
        ticks += 1;

        let (page_size, commitment) = if ticks.is_multiple_of(RECONCILE_EVERY) {
            (RECONCILE_PAGE_SIZE, "finalized")
        } else {
            (FAST_TICK_PAGE_SIZE, "confirmed")
        };

        match process::run_pass(&db, &mut redis, &rpc, &cursor, page_size, commitment).await {
            Ok(processed) if processed > 0 => {
                tracing::info!(processed, commitment, "indexed new transactions");
            }
            Ok(_) => {}
            Err(error) => {
                tracing::error!(%error, commitment, "indexing pass failed, will retry next tick");
            }
        }
    }
}
