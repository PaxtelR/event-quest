use chrono::{DateTime, Utc};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};

use crate::error::ApiError;

use super::token::generate_token;

const SESSION_KEY_PREFIX: &str = "eventquest:session:";
const NONCE_KEY_PREFIX: &str = "eventquest:auth-nonce:";
const SESSION_ID_BYTES: usize = 32;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionData {
    pub wallet: String,
    pub created_at: DateTime<Utc>,
}

/// Stores a freshly issued auth nonce — Redis TTL is the authoritative
/// expiry (spec §9.1); the message's own "Expiration Time" field is
/// checked separately, in `auth::handlers::verify`, as defense in depth.
pub async fn store_nonce(
    redis: &mut redis::aio::ConnectionManager,
    nonce: &str,
    ttl_seconds: u64,
) -> Result<(), ApiError> {
    let key = format!("{NONCE_KEY_PREFIX}{nonce}");
    redis.set_ex::<_, _, ()>(&key, 1, ttl_seconds).await?;
    Ok(())
}

/// Atomically checks and consumes a nonce (`GETDEL`), making it single-use
/// — a concurrent second `verify` request with the same nonce cannot both
/// succeed, per spec §9.1's single-use requirement.
pub async fn consume_nonce(
    redis: &mut redis::aio::ConnectionManager,
    nonce: &str,
) -> Result<bool, ApiError> {
    let key = format!("{NONCE_KEY_PREFIX}{nonce}");
    let existed: Option<i64> = redis::cmd("GETDEL").arg(&key).query_async(redis).await?;
    Ok(existed.is_some())
}

pub async fn create_session(
    redis: &mut redis::aio::ConnectionManager,
    wallet: &str,
    ttl_seconds: u64,
) -> Result<String, ApiError> {
    let session_id = generate_token(SESSION_ID_BYTES);
    let key = format!("{SESSION_KEY_PREFIX}{session_id}");
    let data = SessionData {
        wallet: wallet.to_string(),
        created_at: Utc::now(),
    };
    let json = serde_json::to_string(&data).expect("SessionData always serializes");
    redis.set_ex::<_, _, ()>(&key, json, ttl_seconds).await?;
    Ok(session_id)
}

pub async fn load_session(
    redis: &mut redis::aio::ConnectionManager,
    session_id: &str,
) -> Result<Option<SessionData>, ApiError> {
    let key = format!("{SESSION_KEY_PREFIX}{session_id}");
    let json: Option<String> = redis.get(&key).await?;
    Ok(json.and_then(|json| serde_json::from_str(&json).ok()))
}

pub async fn delete_session(
    redis: &mut redis::aio::ConnectionManager,
    session_id: &str,
) -> Result<(), ApiError> {
    let key = format!("{SESSION_KEY_PREFIX}{session_id}");
    redis.del::<_, ()>(&key).await?;
    Ok(())
}
