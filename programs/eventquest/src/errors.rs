use anchor_lang::prelude::*;

#[error_code]
pub enum EventQuestError {
    #[msg("Arithmetic overflow occurred")]
    Overflow,
    #[msg("Invalid event period: starts_at must be before ends_at")]
    InvalidEventPeriod,
    #[msg("Checkpoint schedule must fall within the event period")]
    CheckpointOutsideEventPeriod,
    #[msg("Invalid checkpoint period: opens_at must be before closes_at")]
    InvalidCheckpointPeriod,
    #[msg("Points must be greater than zero and within the allowed maximum")]
    PointsOutOfRange,
    #[msg("Checkpoint points cannot be changed after check-ins have occurred")]
    CheckpointPointsLocked,
    #[msg("Event is not active")]
    EventNotActive,
    #[msg("Checkpoint is not active")]
    CheckpointNotActive,
    #[msg("Account does not belong to the expected event")]
    EventMismatch,
    #[msg("Current time is outside the checkpoint's open window")]
    CheckpointNotOpen,
    #[msg("Unexpected attestor for this checkpoint")]
    InvalidAttestor,
    #[msg("Only the event authority can perform this action")]
    Unauthorized,
    #[msg("Challenge hash must be a non-zero 32-byte value")]
    InvalidChallengeHash,
    #[msg("This status transition is not allowed")]
    InvalidStatusTransition,
}
