//! `GET /health/live` and `GET /health/ready` per spec §21. Readiness
//! checks every dependency the API actually needs to serve traffic
//! correctly: Postgres, Redis, the Solana RPC endpoint, and access to the
//! attestor signer (without performing a real signature).

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde_json::json;

use crate::state::AppState;

pub async fn live() -> impl IntoResponse {
    (StatusCode::OK, Json(json!({ "status": "live" })))
}

pub async fn ready(State(state): State<AppState>) -> impl IntoResponse {
    let postgres_ok = sqlx::query("select 1").execute(&state.db).await.is_ok();

    let mut redis_conn = state.redis.clone();
    let redis_ok = redis::cmd("PING")
        .query_async::<String>(&mut redis_conn)
        .await
        .is_ok();

    let rpc_ok = check_rpc_health(&state.config.solana.rpc_http_url).await;

    // Confirm the attestor keypair file is reachable without performing a
    // real signature — spec §21: "Acesso ao signer, sem executar assinatura
    // real".
    let signer_ok = tokio::fs::metadata(&state.config.solana.attestor_keypair_path)
        .await
        .is_ok();

    let ready = postgres_ok && redis_ok && rpc_ok && signer_ok;
    let status = if ready {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (
        status,
        Json(json!({
            "status": if ready { "ready" } else { "not_ready" },
            "checks": {
                "postgres": postgres_ok,
                "redis": redis_ok,
                "solanaRpc": rpc_ok,
                "signer": signer_ok,
            }
        })),
    )
}

async fn check_rpc_health(rpc_http_url: &str) -> bool {
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()
    {
        Ok(client) => client,
        Err(_) => return false,
    };

    let response = client
        .post(rpc_http_url)
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getHealth",
        }))
        .send()
        .await;

    matches!(response, Ok(resp) if resp.status().is_success())
}
