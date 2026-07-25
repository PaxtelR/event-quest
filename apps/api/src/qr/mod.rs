//! Rotating QR display — spec §8. Tokens are short-lived, signed HS256
//! JWTs (spec §8.3's MVP recommendation) that never contain a prepared
//! transaction or a secret; the display screen polls or streams the
//! current one via the `/public/checkpoints/{id}/...` routes, gated by a
//! `displayAccessToken` issued only to authenticated organization members.

mod claims;
mod handlers;
mod rotation;

pub(crate) use claims::{verify, QrClaims, QR_SCOPE, QR_VERSION};
pub(crate) use rotation::qr_jti_claims;
// Only `checkins::devnet_tests` needs these, to mint a QR token the same
// way the production `current_qr` handler does.
#[cfg(test)]
pub(crate) use rotation::{get_or_create_current_qr, ActiveCheckpoint};

use axum::routing::{get, post};
use axum::Router;

use crate::state::AppState;

/// Authenticated routes: issuing a display token.
pub fn router() -> Router<AppState> {
    Router::new().route(
        "/checkpoints/{checkpointId}/display-token",
        post(handlers::create_display_token),
    )
}

/// Public routes (still gated by `displayAccessToken`, just not by a
/// wallet session — the checkpoint display screen is typically a kiosk/TV
/// with no authenticated browser session of its own).
pub fn public_router() -> Router<AppState> {
    Router::new()
        .route(
            "/checkpoints/{checkpointId}/current-qr",
            get(handlers::current_qr),
        )
        .route(
            "/checkpoints/{checkpointId}/qr-stream",
            get(handlers::qr_stream),
        )
}
