//! Organization membership lookup for the authenticated wallet. Spec
//! §12.3 never lists an endpoint for this, but the admin frontend has no
//! other way to discover which `organizationId` to pass to `GET /events`
//! for its *own* (including draft/private) events — `POST /events`
//! auto-provisions a personal organization (see
//! `authz::ensure_personal_organization`) the first time a wallet creates
//! an event, but nothing let the wallet look that org back up afterward.
//! Mirrors the pragmatic-gap-filling precedent already set there rather
//! than blocking the admin UI on an unspecified flow.

use axum::routing::get;
use axum::Json;
use axum::Router;
use serde::Serialize;
use uuid::Uuid;

use crate::auth::AuthenticatedWallet;
use crate::error::ApiError;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/organizations/me", get(list_my_organizations))
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct OrganizationMembership {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub role: String,
}

pub async fn list_my_organizations(
    axum::extract::State(state): axum::extract::State<AppState>,
    AuthenticatedWallet(wallet): AuthenticatedWallet,
) -> Result<Json<Vec<OrganizationMembership>>, ApiError> {
    let memberships = sqlx::query_as::<_, OrganizationMembership>(
        "select o.id, o.name, o.slug, m.role::text as role \
         from organization_members m \
         join organizations o on o.id = m.organization_id \
         where m.wallet_address = $1 \
         order by m.created_at asc",
    )
    .bind(&wallet)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(memberships))
}
