use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidateRequest {
    pub qr_token: String,
    pub wallet: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GrantResponse {
    pub grant_id: Uuid,
    pub event_id: Uuid,
    pub checkpoint_id: Uuid,
    pub wallet: String,
    pub challenge_hash: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrepareTransactionResponse {
    pub transaction: String,
    pub network: &'static str,
    pub expires_at: DateTime<Utc>,
    pub grant_id: Uuid,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmittedRequest {
    pub signature: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckinStatusResponse {
    pub grant_id: Uuid,
    pub status: String,
    pub transaction_signature: Option<String>,
    pub failure_code: Option<String>,
    pub failure_message: Option<String>,
    pub expires_at: DateTime<Utc>,
}
