//! Shared domain types for EventQuest, used by both `apps/api` and
//! `apps/indexer`: the standardized error taxonomy (spec §20), typed
//! entity IDs, and the off-chain check-in state machine (spec §9.5).
//! Framework-agnostic on purpose — no axum, no sqlx — so it stays usable
//! from either service without pulling in the other's dependencies.

mod checkin;
mod error;
mod ids;

pub use checkin::CheckinAttemptStatus;
pub use error::{ApiErrorBody, ErrorCode};
pub use ids::{CheckpointId, EventId, GrantId, OrganizationId, ParticipantId};
