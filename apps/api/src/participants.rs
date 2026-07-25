//! Participant-facing progress (spec §12.3 "Participante") and the
//! organizer-facing participants list for a single event. Spec §12.3
//! lists `GET /me/events` (and drill-down routes this module doesn't
//! implement yet: `/me/events/{id}/progress`, `/me/events/{id}/attendances`,
//! `/me/transactions`) and never lists an event-scoped participants
//! listing at all — added here for the same reason as
//! `organizations.rs`: the admin participants/analytics pages have no
//! other real data source, and a mocked one would violate this project's
//! own "no mocks" rule.

use axum::extract::{Path, State};
use axum::routing::get;
use axum::Json;
use axum::Router;
use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

use crate::auth::AuthenticatedWallet;
use crate::authz;
use crate::error::ApiError;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/me/events", get(my_events))
        .route("/events/{eventId}/participants", get(event_participants))
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct MyEventProgress {
    pub event_id: Uuid,
    pub event_name: String,
    pub event_slug: String,
    pub points: i64,
    pub checkin_count: i32,
    pub completed: bool,
    pub joined_at: DateTime<Utc>,
}

pub async fn my_events(
    State(state): State<AppState>,
    AuthenticatedWallet(wallet): AuthenticatedWallet,
) -> Result<Json<Vec<MyEventProgress>>, ApiError> {
    let progress = sqlx::query_as::<_, MyEventProgress>(
        "select e.id as event_id, e.name as event_name, e.slug as event_slug, \
         ep.points, ep.checkin_count, ep.completed, ep.joined_at \
         from event_participants ep \
         join participants p on p.id = ep.participant_id \
         join events e on e.id = ep.event_id \
         where p.wallet_address = $1 \
         order by ep.joined_at desc",
    )
    .bind(&wallet)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(progress))
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct EventParticipant {
    pub participant_id: Uuid,
    pub wallet_address: String,
    pub points: i64,
    pub checkin_count: i32,
    pub completed: bool,
    pub joined_at: DateTime<Utc>,
}

pub async fn event_participants(
    State(state): State<AppState>,
    AuthenticatedWallet(wallet): AuthenticatedWallet,
    Path(event_id): Path<Uuid>,
) -> Result<Json<Vec<EventParticipant>>, ApiError> {
    let organization_id = authz::event_organization_id(&state.db, event_id).await?;
    authz::require_org_member(&state.db, organization_id, &wallet).await?;

    let participants = sqlx::query_as::<_, EventParticipant>(
        "select p.id as participant_id, p.wallet_address, ep.points, ep.checkin_count, \
         ep.completed, ep.joined_at \
         from event_participants ep \
         join participants p on p.id = ep.participant_id \
         where ep.event_id = $1 \
         order by ep.points desc, ep.joined_at asc",
    )
    .bind(event_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(participants))
}
