use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckpointDto {
    pub id: Uuid,
    pub event_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub points: i32,
    pub rotation_seconds: i32,
    pub opens_at: DateTime<Utc>,
    pub closes_at: DateTime<Utc>,
    pub status: String,
    pub attestor_pubkey: String,
    pub onchain_checkpoint_address: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub const CHECKPOINT_COLUMNS: &str = "id, event_id, name, description, points, rotation_seconds, \
     opens_at, closes_at, status::text as status, attestor_pubkey, \
     onchain_checkpoint_address, created_at, updated_at";

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateCheckpointRequest {
    #[validate(length(min = 1, max = 200))]
    pub name: String,
    pub description: Option<String>,
    #[validate(range(min = 1, message = "points must be greater than zero"))]
    pub points: i32,
    #[validate(range(
        min = 10,
        max = 60,
        message = "rotationSeconds must be between 10 and 60"
    ))]
    pub rotation_seconds: Option<i32>,
    pub opens_at: DateTime<Utc>,
    pub closes_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCheckpointRequest {
    #[validate(length(min = 1, max = 200))]
    pub name: Option<String>,
    pub description: Option<String>,
    #[validate(range(min = 1, message = "points must be greater than zero"))]
    pub points: Option<i32>,
    #[validate(range(
        min = 10,
        max = 60,
        message = "rotationSeconds must be between 10 and 60"
    ))]
    pub rotation_seconds: Option<i32>,
    pub opens_at: Option<DateTime<Utc>>,
    pub closes_at: Option<DateTime<Utc>>,
}
