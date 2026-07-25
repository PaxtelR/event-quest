use chrono::Utc;
use eventquest_domain::ErrorCode;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::generate_token;
use crate::error::ApiError;
use crate::state::AppState;

use super::claims::{sign, QrClaims, QR_SCOPE, QR_VERSION};

const QR_JTI_ENTROPY_BYTES: usize = 16; // 128 bits, spec §8.2's minimum.

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QrWindow {
    pub token: String,
    pub qr_url: String,
    pub issued_at: i64,
    pub expires_at: i64,
}

/// `floor(now / (rotationSeconds * 1000))` — spec §8.5's `getRotationWindow`.
/// All checkpoint displays showing the same checkpoint land on the same
/// window index and therefore the same cached QR.
fn rotation_window(now_ms: i64, rotation_seconds: i64) -> i64 {
    now_ms / (rotation_seconds * 1000)
}

pub struct ActiveCheckpoint {
    pub event_id: Uuid,
    pub rotation_seconds: i64,
}

/// Loads the checkpoint, confirming it's active — mirrors spec §8.5's
/// `loadActiveCheckpoint`. An inactive/nonexistent checkpoint means the
/// display should show "Check-in temporarily unavailable" (spec §15.4),
/// never a stale QR.
pub async fn load_active_checkpoint(
    state: &AppState,
    checkpoint_id: Uuid,
) -> Result<ActiveCheckpoint, ApiError> {
    let row: Option<(Uuid, i32, String)> = sqlx::query_as(
        "select event_id, rotation_seconds, status::text as status \
         from checkpoints where id = $1",
    )
    .bind(checkpoint_id)
    .fetch_optional(&state.db)
    .await?;

    match row {
        Some((event_id, rotation_seconds, status)) if status == "active" => Ok(ActiveCheckpoint {
            event_id,
            rotation_seconds: rotation_seconds as i64,
        }),
        Some(_) => Err(ApiError::new(ErrorCode::CheckpointNotActive)),
        None => Err(ApiError::new(ErrorCode::ResourceNotFound)),
    }
}

/// Spec §8.5's `getOrCreateCurrentQr`: returns the same token for every
/// caller within the same rotation window, generating a fresh one only
/// once per window (via Redis `SET NX`, so concurrent requests racing to
/// create the same window's QR converge on one winner).
pub async fn get_or_create_current_qr(
    state: &AppState,
    checkpoint_id: Uuid,
    checkpoint: &ActiveCheckpoint,
) -> Result<QrWindow, ApiError> {
    let mut redis = state.redis.clone();

    let now_ms = Utc::now().timestamp_millis();
    let window = rotation_window(now_ms, checkpoint.rotation_seconds);
    let cache_key = format!("eventquest:qr-window:{checkpoint_id}:{window}");
    let ttl = checkpoint.rotation_seconds as u64 + state.config.qr.grace_seconds + 10;

    if let Some(existing) = read_cached_window(&mut redis, &cache_key).await? {
        return Ok(existing);
    }

    let issued_at = window * checkpoint.rotation_seconds;
    let expires_at = issued_at + checkpoint.rotation_seconds;
    let jti = generate_token(QR_JTI_ENTROPY_BYTES);

    let claims = QrClaims {
        v: QR_VERSION,
        scope: QR_SCOPE.to_string(),
        event_id: checkpoint.event_id,
        checkpoint_id,
        jti: jti.clone(),
        iat: issued_at,
        exp: expires_at,
    };

    let token = sign(&claims, &state.config.qr.signing_secret)
        .map_err(|_| ApiError::new(ErrorCode::InternalError))?;
    let qr_url = format!(
        "{}/check-in?token={}",
        state.config.public_app_url,
        urlencoding::encode(&token)
    );

    let result = QrWindow {
        token,
        qr_url,
        issued_at,
        expires_at,
    };
    let result_json = serde_json::to_string(&result).expect("QrWindow always serializes");

    let won_race: Option<String> = redis::cmd("SET")
        .arg(&cache_key)
        .arg(&result_json)
        .arg("EX")
        .arg(ttl)
        .arg("NX")
        .query_async(&mut redis)
        .await?;

    if won_race.is_none() {
        // Someone else created this window's QR between our GET and SET —
        // use their result instead of ours so every display agrees.
        if let Some(existing) = read_cached_window(&mut redis, &cache_key).await? {
            return Ok(existing);
        }
    }

    let jti_key = format!("eventquest:qr:{checkpoint_id}:{jti}");
    let claims_json = serde_json::to_string(&claims).expect("QrClaims always serializes");
    redis.set_ex::<_, _, ()>(&jti_key, claims_json, ttl).await?;

    Ok(result)
}

async fn read_cached_window(
    redis: &mut redis::aio::ConnectionManager,
    cache_key: &str,
) -> Result<Option<QrWindow>, ApiError> {
    let cached: Option<String> = redis.get(cache_key).await?;
    Ok(cached.and_then(|json| serde_json::from_str(&json).ok()))
}

/// Looks up the claims stored for a specific QR `jti` (set by
/// `get_or_create_current_qr` above) — spec §9.2 validations #12 ("Token
/// encontrado no Redis") and #13 ("QR não revogado"): this project has no
/// separate revocation list, so a jti that isn't in Redis (never issued,
/// naturally expired, or would-be-revoked) is exactly the same as an
/// invalid one — there's nothing else to check.
pub async fn qr_jti_claims(
    state: &AppState,
    checkpoint_id: Uuid,
    jti: &str,
) -> Result<Option<QrClaims>, ApiError> {
    let mut redis = state.redis.clone();
    let key = format!("eventquest:qr:{checkpoint_id}:{jti}");
    let cached: Option<String> = redis.get(&key).await?;
    Ok(cached.and_then(|json| serde_json::from_str(&json).ok()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotation_window_groups_timestamps_into_fixed_buckets() {
        let rotation_seconds = 15;
        let bucket_ms = rotation_seconds * 1000;

        let w1 = rotation_window(0, rotation_seconds);
        let w2 = rotation_window(bucket_ms - 1, rotation_seconds);
        assert_eq!(w1, w2, "same bucket");

        let w3 = rotation_window(bucket_ms, rotation_seconds);
        assert_eq!(w3, w1 + 1, "next bucket");
    }
}
