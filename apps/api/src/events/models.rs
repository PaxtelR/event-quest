use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventDto {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub banner_url: Option<String>,
    pub location_name: Option<String>,
    pub location_address: Option<String>,
    pub timezone: String,
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
    pub status: String,
    pub visibility: String,
    pub solana_network: String,
    pub onchain_event_address: Option<String>,
    pub onchain_create_signature: Option<String>,
    pub created_by_wallet: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Every column `EventDto` needs, with enum columns cast to `text` so
/// `sqlx::FromRow` doesn't need custom `Type` impls for the Postgres enums.
pub const EVENT_COLUMNS: &str = "id, organization_id, name, slug, description, banner_url, \
     location_name, location_address, timezone, starts_at, ends_at, \
     status::text as status, visibility::text as visibility, \
     solana_network::text as solana_network, onchain_event_address, \
     onchain_create_signature, created_by_wallet, created_at, updated_at";

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateEventRequest {
    #[validate(length(min = 1, max = 200))]
    pub name: String,
    #[validate(length(min = 1, max = 200, message = "slug must be 1-200 characters"))]
    pub slug: String,
    pub description: Option<String>,
    pub banner_url: Option<String>,
    pub location_name: Option<String>,
    pub location_address: Option<String>,
    pub timezone: Option<String>,
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
    pub visibility: Option<EventVisibilityInput>,
    pub organization_id: Option<Uuid>,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum EventVisibilityInput {
    Public,
    Private,
}

impl EventVisibilityInput {
    pub fn as_db_str(self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Private => "private",
        }
    }
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UpdateEventRequest {
    #[validate(length(min = 1, max = 200))]
    pub name: Option<String>,
    pub description: Option<String>,
    pub banner_url: Option<String>,
    pub location_name: Option<String>,
    pub location_address: Option<String>,
    pub timezone: Option<String>,
    pub starts_at: Option<DateTime<Utc>>,
    pub ends_at: Option<DateTime<Utc>>,
    pub visibility: Option<EventVisibilityInput>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListEventsQuery {
    pub organization_id: Option<Uuid>,
}
