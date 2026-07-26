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
mod organizations;
mod participants;
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

    // `RUST_LOG` (tracing's own convention) wins if set; otherwise fall
    // back to this project's `LOG_LEVEL` (documented in .env.example),
    // then "info". Config isn't loaded yet at this point (logging must be
    // ready before anything else can report errors), so this reads the
    // env var directly rather than through `AppConfig`.
    let log_filter = std::env::var("RUST_LOG")
        .or_else(|_| std::env::var("LOG_LEVEL"))
        .unwrap_or_else(|_| "info".to_string());
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::new(log_filter))
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

    let bind_addr = bind_address(std::env::var("PORT").ok().as_deref(), &config.api_url);
    let state = state::AppState::new(config, db, redis, attestor_pubkey, blockchain);
    let app = router::build(state);

    tracing::info!(%bind_addr, "listening");
    let listener = tokio::net::TcpListener::bind(bind_addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// Binds on all interfaces. Prefers `PORT` (the convention platforms like
/// Railway/Heroku inject at runtime with a dynamically assigned value the
/// service *must* listen on for their healthcheck/routing to find it),
/// falling back to the port embedded in `API_URL` — which is what local
/// dev and docker-compose rely on, since neither sets `PORT` — and finally
/// to 3001 if neither parses.
fn bind_address(port_env: Option<&str>, api_url: &str) -> SocketAddr {
    let port = port_env
        .and_then(|value| value.parse().ok())
        .or_else(|| url::Url::parse(api_url).ok().and_then(|url| url.port()))
        .unwrap_or(3001);
    SocketAddr::new(IpAddr::from([0, 0, 0, 0]), port)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bind_address_uses_port_from_api_url() {
        assert_eq!(bind_address(None, "http://localhost:3001").port(), 3001);
        assert_eq!(bind_address(None, "http://localhost:8080").port(), 8080);
        assert_eq!(bind_address(None, "not a url").port(), 3001);
    }

    #[test]
    fn bind_address_prefers_the_port_env_var_when_set() {
        // Railway (and most PaaS platforms) inject a dynamically assigned
        // `PORT` the service must listen on for healthchecks to find it —
        // this must win over whatever port happens to be in `API_URL`.
        assert_eq!(
            bind_address(Some("8080"), "http://localhost:3001").port(),
            8080
        );
        // Garbage `PORT` values fall back rather than binding to port 0.
        assert_eq!(
            bind_address(Some("not a number"), "http://localhost:3001").port(),
            3001
        );
    }
}
