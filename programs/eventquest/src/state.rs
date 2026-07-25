use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace, Debug)]
pub struct EventAccount {
    pub authority: Pubkey,
    pub external_id_hash: [u8; 32],
    pub starts_at: i64,
    pub ends_at: i64,
    pub status: EventStatus,
    pub checkpoint_count: u32,
    pub total_checkins: u64,
    pub bump: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, InitSpace, Clone, Copy, PartialEq, Eq, Debug)]
pub enum EventStatus {
    Draft,
    Active,
    Paused,
    Finished,
    Cancelled,
}

impl EventStatus {
    /// Finished and cancelled events cannot transition to any other status.
    pub fn is_terminal(&self) -> bool {
        matches!(self, EventStatus::Finished | EventStatus::Cancelled)
    }
}

#[account]
#[derive(InitSpace, Debug)]
pub struct CheckpointAccount {
    pub event: Pubkey,
    pub attestor: Pubkey,
    pub external_id_hash: [u8; 32],
    pub opens_at: i64,
    pub closes_at: i64,
    pub points: u32,
    pub active: bool,
    pub total_checkins: u64,
    pub bump: u8,
}

#[account]
#[derive(InitSpace, Debug)]
pub struct ParticipantEventAccount {
    pub event: Pubkey,
    pub participant: Pubkey,
    pub points: u64,
    pub checkin_count: u32,
    pub completed: bool,
    pub created_at: i64,
    pub updated_at: i64,
    pub bump: u8,
}

#[account]
#[derive(InitSpace, Debug)]
pub struct AttendanceAccount {
    pub event: Pubkey,
    pub checkpoint: Pubkey,
    pub participant: Pubkey,
    pub attestor: Pubkey,
    pub challenge_hash: [u8; 32],
    pub checked_in_at: i64,
    pub points_awarded: u32,
    pub bump: u8,
}
