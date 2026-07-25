//! Shared plumbing for organizer-signed on-chain actions — provisioning
//! events/checkpoints and syncing their lifecycle status (spec §11.3's
//! `initialize_event`/`update_event_status`/`create_checkpoint`/
//! `update_checkpoint`). Used by both `events` and `checkpoints`, which is
//! why it lives at the crate root rather than inside either module.
//!
//! Unlike the check-ins flow (`checkins::grant`), there's no adversarial
//! multi-party validation here — the caller is already an authenticated,
//! authorized organizer — so there's no Redis-backed grant. Everything
//! `prepare` needs is deterministically recomputed from the resource's own
//! id (see `external_id_hash`), and `submitted` re-derives the same PDA to
//! verify against, so no state has to survive between the two calls.

use std::time::Duration;

use anchor_lang::prelude::Pubkey;
use anchor_lang::AccountDeserialize;
use eventquest_domain::ErrorCode;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::error::ApiError;
use crate::state::AppState;

/// How long to wait for an organizer's on-chain action to confirm before
/// giving up. Generous compared to the check-in flow's checks: this is a
/// low-frequency admin action, not a hot path, and the off-chain status
/// transition is deliberately gated on this confirmation (see this
/// module's header comment) rather than left to the indexer.
pub const CONFIRM_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmittedRequest {
    pub signature: String,
}

/// A deterministic 32-byte identifier derived from the resource's own
/// UUID — this project's convention (mirrored from the QR/challenge
/// hashing elsewhere) for giving an off-chain id an on-chain
/// representation without ever putting the raw UUID on-chain. Never
/// needs to be stored: both `prepare` and `submitted` recompute it from
/// the same `id` path parameter.
pub fn external_id_hash(id: Uuid) -> [u8; 32] {
    Sha256::digest(id.as_bytes()).into()
}

/// Response shape for every `.../prepare-transaction` endpoint in this
/// family: either the action needed no on-chain step and was already
/// applied off-chain (`requires_signature: false`, `resource` populated),
/// or the organizer's wallet needs to sign `transaction` first (`resource`
/// absent — the caller gets it back from the matching `submitted` call).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrepareResponse<T> {
    pub requires_signature: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transaction: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource: Option<T>,
}

impl<T> PrepareResponse<T> {
    pub fn applied(resource: T) -> Self {
        Self {
            requires_signature: false,
            transaction: None,
            network: None,
            resource: Some(resource),
        }
    }

    pub fn requires_signature(transaction: String) -> Self {
        Self {
            requires_signature: true,
            transaction: Some(transaction),
            network: Some("devnet"),
            resource: None,
        }
    }
}

/// Validates `signature`'s format, waits for confirmation, and fetches the
/// account at `pda` — the shared "did the organizer's signed transaction
/// actually do what we expect" check every `submitted` handler in this
/// family runs before applying its off-chain transition.
pub async fn confirm_and_fetch<T: AccountDeserialize>(
    state: &AppState,
    signature: &str,
    pda: Pubkey,
) -> Result<T, ApiError> {
    let signature_bytes = bs58::decode(signature)
        .into_vec()
        .map_err(|_| ApiError::with_message(ErrorCode::ValidationError, "invalid signature"))?;
    if signature_bytes.len() != 64 {
        return Err(ApiError::with_message(
            ErrorCode::ValidationError,
            "invalid signature",
        ));
    }

    let confirmed = state.blockchain.confirm(signature, CONFIRM_TIMEOUT).await?;
    if !confirmed {
        return Err(ApiError::new(ErrorCode::TransactionFailed));
    }

    state
        .blockchain
        .fetch_account::<T>(pda)
        .await?
        .ok_or_else(|| ApiError::new(ErrorCode::TransactionFailed))
}
