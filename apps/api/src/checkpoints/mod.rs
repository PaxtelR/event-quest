//! Checkpoint management — spec §12.3 "Checkpoints" and §5.2/§5.3
//! organizer/operator capabilities.
//!
//! `activate`/`pause` each need a `prepare-transaction` + `submitted` pair
//! rather than spec §12.3's single-call shape: `check_in` validates
//! `checkpoint.active` against the ON-CHAIN account, so the off-chain
//! status can't just flip on its own — see apps/api/src/onchain.rs and
//! this module's `handlers::prepare_activate` for why (the organizer
//! on-chain-provisioning gap noted in apps/api/src/checkins/mod.rs).

pub(crate) mod handlers;
pub(crate) mod models;

use axum::routing::{get, post};
use axum::Router;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/events/{eventId}/checkpoints",
            post(handlers::create_checkpoint).get(handlers::list_checkpoints),
        )
        .route(
            "/checkpoints/{checkpointId}",
            get(handlers::get_checkpoint).patch(handlers::update_checkpoint),
        )
        .route(
            "/checkpoints/{checkpointId}/activate/prepare-transaction",
            post(handlers::prepare_activate),
        )
        .route(
            "/checkpoints/{checkpointId}/activate/submitted",
            post(handlers::activate_submitted),
        )
        .route(
            "/checkpoints/{checkpointId}/pause/prepare-transaction",
            post(handlers::prepare_pause),
        )
        .route(
            "/checkpoints/{checkpointId}/pause/submitted",
            post(handlers::pause_submitted),
        )
}
