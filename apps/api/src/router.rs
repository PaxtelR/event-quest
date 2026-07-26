use axum::extract::Request;
use axum::routing::get;
use axum::Router;
use tower_http::trace::TraceLayer;

use crate::{
    auth, checkins, checkpoints, events, health, organizations, participants, qr, state::AppState,
};

pub fn build(state: AppState) -> Router {
    let api_v1 = Router::new()
        .merge(auth::router())
        .merge(events::router())
        .merge(checkpoints::router())
        .merge(checkins::router())
        .merge(organizations::router())
        .merge(participants::router())
        .merge(qr::router())
        .nest("/public", qr::public_router());

    Router::new()
        .route("/health/live", get(health::live))
        .route("/health/ready", get(health::ready))
        .nest("/api/v1", api_v1)
        // Default `TraceLayer::new_for_http()` records the full request URI
        // — including query strings — as a span field. Several routes here
        // carry bearer-like credentials in the query string (QR check-in
        // tokens, `displayAccessToken`) because they're consumed by
        // `EventSource`/plain `GET`, which can't send custom headers; the
        // spec explicitly requires these never be logged in full (§26). Span
        // from method + path only, never the query string.
        .layer(TraceLayer::new_for_http().make_span_with(|request: &Request| {
            tracing::info_span!("http_request", method = %request.method(), path = %request.uri().path())
        }))
        .with_state(state)
}
