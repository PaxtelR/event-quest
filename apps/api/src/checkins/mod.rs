//! Secure check-in flow — spec §9.2-§9.5: validate a scanned QR into a
//! short-lived per-wallet grant, prepare an attestor-co-signed transaction,
//! and record the participant's submitted signature. `apps/indexer` (Phase
//! 3 task #19) is the actual source of truth for the final `CONFIRMED`
//! state (spec §16) — this module only gets an attempt as far as
//! `TRANSACTION_SUBMITTED`, plus a fast-path failure if one is immediately
//! observable.

mod grant;
pub(crate) mod handlers;
pub(crate) mod models;

#[cfg(test)]
mod devnet_tests;

use axum::routing::{get, post};
use axum::Router;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/check-ins/validate", post(handlers::validate))
        .route(
            "/check-ins/{grantId}/prepare-transaction",
            post(handlers::prepare_transaction),
        )
        .route("/check-ins/{grantId}/submitted", post(handlers::submitted))
        .route("/check-ins/{grantId}", get(handlers::get_status))
}
