//! Event management — spec §12.3 "Eventos" and §5.2 organizer
//! capabilities.
//!
//! Spec §12.3 lists event/checkpoint endpoints but never an explicit
//! "create organization" one, even though `events.organization_id` is
//! required. Rather than block the vertical slice on an unspecified flow,
//! `POST /events` auto-provisions a personal organization for the caller's
//! wallet the first time they create an event (see
//! `authz::ensure_personal_organization`) — a normal pattern for
//! single-tenant-by-default SaaS onboarding. An explicit `organizationId`
//! can still be passed once multi-admin organizations are managed some
//! other way.
//!
//! `publish`/`pause`/`finish` each need a `prepare-transaction` +
//! `submitted` pair rather than spec §12.3's single-call shape: `check_in`
//! validates `event.status == Active` against the ON-CHAIN account, so
//! the off-chain status can't just flip on its own — see
//! apps/api/src/onchain.rs and this module's `handlers::prepare_publish`
//! for why (the organizer on-chain-provisioning gap noted in
//! apps/api/src/checkins/mod.rs).

pub(crate) mod handlers;
pub(crate) mod models;

use axum::routing::{get, post};
use axum::Router;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/events",
            post(handlers::create_event).get(handlers::list_events),
        )
        .route(
            "/events/{eventId}",
            get(handlers::get_event).patch(handlers::update_event),
        )
        .route(
            "/events/{eventId}/publish/prepare-transaction",
            post(handlers::prepare_publish),
        )
        .route(
            "/events/{eventId}/publish/submitted",
            post(handlers::publish_submitted),
        )
        .route(
            "/events/{eventId}/pause/prepare-transaction",
            post(handlers::prepare_pause),
        )
        .route(
            "/events/{eventId}/pause/submitted",
            post(handlers::pause_submitted),
        )
        .route(
            "/events/{eventId}/finish/prepare-transaction",
            post(handlers::prepare_finish),
        )
        .route(
            "/events/{eventId}/finish/submitted",
            post(handlers::finish_submitted),
        )
}
