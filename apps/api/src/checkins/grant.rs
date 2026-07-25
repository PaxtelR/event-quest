//! Redis storage for the individual check-in authorization created by
//! `validate` — spec §9.2: `eventquest:checkin-grant:{grantId}`, bound to
//! wallet, event, checkpoint, QR hash, and idempotency key so it can't be
//! reused by another wallet or replayed against a different checkpoint.

use chrono::{DateTime, Utc};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::ApiError;

const GRANT_KEY_PREFIX: &str = "eventquest:checkin-grant:";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckinGrant {
    pub grant_id: Uuid,
    pub event_id: Uuid,
    pub checkpoint_id: Uuid,
    pub participant_id: Uuid,
    pub wallet: String,
    pub challenge_hash: String,
    pub idempotency_key: String,
    pub expires_at: DateTime<Utc>,
}

pub async fn store_grant(
    redis: &mut redis::aio::ConnectionManager,
    grant: &CheckinGrant,
    ttl_seconds: u64,
) -> Result<(), ApiError> {
    let key = format!("{GRANT_KEY_PREFIX}{}", grant.grant_id);
    let json = serde_json::to_string(grant).expect("CheckinGrant always serializes");
    redis.set_ex::<_, _, ()>(&key, json, ttl_seconds).await?;
    Ok(())
}

pub async fn load_grant(
    redis: &mut redis::aio::ConnectionManager,
    grant_id: Uuid,
) -> Result<Option<CheckinGrant>, ApiError> {
    let key = format!("{GRANT_KEY_PREFIX}{grant_id}");
    let json: Option<String> = redis.get(&key).await?;
    Ok(json.and_then(|json| serde_json::from_str(&json).ok()))
}
