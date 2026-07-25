use std::convert::Infallible;
use std::time::Duration;

use axum::extract::{Path, Query, State};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::Json;
use chrono::{DateTime, TimeZone, Utc};
use eventquest_domain::ErrorCode;
use futures_util::Stream;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::AuthenticatedWallet;
use crate::authz;
use crate::error::ApiError;
use crate::state::AppState;

use super::claims::{sign, verify, DisplayClaims, DISPLAY_SCOPE};
use super::rotation::{get_or_create_current_qr, load_active_checkpoint, QrWindow};

/// The display token lives long enough for a checkpoint operator to leave
/// the display screen open for an entire event day without re-issuing it.
const DISPLAY_TOKEN_TTL_SECONDS: i64 = 12 * 60 * 60;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DisplayTokenResponse {
    pub display_access_token: String,
    pub expires_at: DateTime<Utc>,
}

pub async fn create_display_token(
    State(state): State<AppState>,
    AuthenticatedWallet(wallet): AuthenticatedWallet,
    Path(checkpoint_id): Path<Uuid>,
) -> Result<Json<DisplayTokenResponse>, ApiError> {
    let organization_id = authz::checkpoint_organization_id(&state.db, checkpoint_id).await?;
    authz::require_org_member(&state.db, organization_id, &wallet).await?;

    let now = Utc::now().timestamp();
    let exp = now + DISPLAY_TOKEN_TTL_SECONDS;
    let claims = DisplayClaims {
        scope: DISPLAY_SCOPE.to_string(),
        checkpoint_id,
        iat: now,
        exp,
    };

    let token = sign(&claims, &state.config.qr.signing_secret)
        .map_err(|_| ApiError::new(ErrorCode::InternalError))?;

    Ok(Json(DisplayTokenResponse {
        display_access_token: token,
        expires_at: Utc.timestamp_opt(exp, 0).single().unwrap_or_else(Utc::now),
    }))
}

#[derive(Debug, Deserialize)]
pub struct DisplayTokenQuery {
    #[serde(rename = "displayAccessToken")]
    pub display_access_token: Option<String>,
}

/// Verifies the caller presented a valid, unexpired display token scoped
/// to exactly this checkpoint — spec §8.4: the public QR routes "deve
/// exigir um displayAccessToken... para evitar que qualquer pessoa
/// descubra checkpoints privados".
fn require_display_access(
    state: &AppState,
    checkpoint_id: Uuid,
    query: &DisplayTokenQuery,
) -> Result<(), ApiError> {
    let token = query
        .display_access_token
        .as_deref()
        .ok_or_else(|| ApiError::new(ErrorCode::AuthRequired))?;

    let claims: DisplayClaims = verify(token, &state.config.qr.signing_secret, 0)
        .map_err(|_| ApiError::new(ErrorCode::AuthRequired))?;

    if claims.scope != DISPLAY_SCOPE || claims.checkpoint_id != checkpoint_id {
        return Err(ApiError::new(ErrorCode::AuthRequired));
    }

    Ok(())
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrentQrResponse {
    pub token: String,
    pub qr_url: String,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub remaining_seconds: i64,
}

impl CurrentQrResponse {
    fn from_window(window: QrWindow) -> Self {
        let now = Utc::now().timestamp();
        Self {
            token: window.token,
            qr_url: window.qr_url,
            issued_at: Utc
                .timestamp_opt(window.issued_at, 0)
                .single()
                .unwrap_or_else(Utc::now),
            expires_at: Utc
                .timestamp_opt(window.expires_at, 0)
                .single()
                .unwrap_or_else(Utc::now),
            remaining_seconds: (window.expires_at - now).max(0),
        }
    }
}

pub async fn current_qr(
    State(state): State<AppState>,
    Path(checkpoint_id): Path<Uuid>,
    Query(query): Query<DisplayTokenQuery>,
) -> Result<Json<CurrentQrResponse>, ApiError> {
    require_display_access(&state, checkpoint_id, &query)?;

    let checkpoint = load_active_checkpoint(&state, checkpoint_id).await?;
    let window = get_or_create_current_qr(&state, checkpoint_id, &checkpoint).await?;

    Ok(Json(CurrentQrResponse::from_window(window)))
}

pub async fn qr_stream(
    State(state): State<AppState>,
    Path(checkpoint_id): Path<Uuid>,
    Query(query): Query<DisplayTokenQuery>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, ApiError> {
    require_display_access(&state, checkpoint_id, &query)?;

    let stream = async_stream::stream! {
        let mut last_token: Option<String> = None;
        let mut interval = tokio::time::interval(Duration::from_secs(1));

        loop {
            interval.tick().await;

            let checkpoint = match load_active_checkpoint(&state, checkpoint_id).await {
                Ok(checkpoint) => checkpoint,
                Err(_) => {
                    // Checkpoint paused/closed: tell the display to stop
                    // showing a QR, without tearing down the connection —
                    // it may become active again.
                    if last_token.take().is_some() {
                        yield Ok(Event::default().event("inactive").data("{}"));
                    }
                    continue;
                }
            };

            let window = match get_or_create_current_qr(&state, checkpoint_id, &checkpoint).await {
                Ok(window) => window,
                Err(_) => continue,
            };

            if last_token.as_deref() != Some(window.token.as_str()) {
                last_token = Some(window.token.clone());
                let payload = CurrentQrResponse::from_window(window);
                let json = serde_json::to_string(&payload).unwrap_or_default();
                yield Ok(Event::default().event("qr").data(json));
            }
        }
    };

    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}
