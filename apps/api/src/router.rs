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
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
