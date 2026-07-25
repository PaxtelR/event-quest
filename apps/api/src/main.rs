//! EventQuest API (Axum). Refuses to start unless `SOLANA_NETWORK=devnet`
//! and Postgres/Redis are reachable — see docs/adr/ADR-001-architecture.md
//! and spec §12.1.

mod auth;
mod authz;
mod blockchain;
mod checkins;
mod checkpoints;
mod error;
mod events;
mod health;
mod onchain;
mod qr;
mod router;
mod state;

#[cfg(test)]
mod organizer_devnet_tests;

use std::net::{IpAddr, SocketAddr};

use eventquest_config::AppConfig;
use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Local dev convenience only; in deployed environments the real
    // environment is injected directly and no `.env` file exists.
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

    tracing::info!(network = %config.solana.network, "starting eventquest-api");

    let db = PgPoolOptions::new()
        .max_connections(10)
        .connect(&config.database_url)
        .await
        .inspect_err(|error| tracing::error!(%error, "failed to connect to Postgres"))?;

    sqlx::migrate!("./migrations")
        .run(&db)
        .await
        .inspect_err(|error| tracing::error!(%error, "failed to apply migrations"))?;

    let redis_client = redis::Client::open(config.redis_url.clone())
        .inspect_err(|error| tracing::error!(%error, "invalid REDIS_URL"))?;
    let redis = redis::aio::ConnectionManager::new(redis_client)
        .await
        .inspect_err(|error| tracing::error!(%error, "failed to connect to Redis"))?;

    let attestor_pubkey = state::load_attestor_pubkey(&config.solana.attestor_keypair_path)
        .await
        .inspect_err(|error| tracing::error!(%error, "failed to load attestor keypair"))?;
    tracing::info!(attestor_pubkey = %attestor_pubkey, "loaded attestor keypair");

    let blockchain = blockchain::BlockchainClient::new(
        &config.solana.rpc_http_url,
        &config.solana.attestor_keypair_path,
        config.solana.program_id,
    )
    .await
    .inspect_err(|error| tracing::error!(%error, "failed to initialize blockchain client"))?;

    let bind_addr = bind_address(&config.api_url);
    let state = state::AppState::new(config, db, redis, attestor_pubkey, blockchain);
    let app = router::build(state);

    tracing::info!(%bind_addr, "listening");
    let listener = tokio::net::TcpListener::bind(bind_addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// Binds on all interfaces, using the port from `API_URL` (defaulting to
/// 3001 if it can't be parsed) so local dev and container deployments agree
/// on the port without a second env var to keep in sync.
fn bind_address(api_url: &str) -> SocketAddr {
    let port = url::Url::parse(api_url)
        .ok()
        .and_then(|url| url.port())
        .unwrap_or(3001);
    SocketAddr::new(IpAddr::from([0, 0, 0, 0]), port)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bind_address_uses_port_from_api_url() {
        assert_eq!(bind_address("http://localhost:3001").port(), 3001);
        assert_eq!(bind_address("http://localhost:8080").port(), 8080);
        assert_eq!(bind_address("not a url").port(), 3001);
    }
}
