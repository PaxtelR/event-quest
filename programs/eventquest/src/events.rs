use anchor_lang::prelude::*;

/// Emitted on every successful `check_in`. This is the only channel the
/// off-chain indexer trusts as proof that attendance was recorded — see
/// docs/adr/ADR-001-architecture.md.
#[event]
pub struct AttendanceRecorded {
    pub event: Pubkey,
    pub checkpoint: Pubkey,
    pub participant: Pubkey,
    pub attestor: Pubkey,
    pub checked_in_at: i64,
    pub points_awarded: u32,
    pub challenge_hash: [u8; 32],
}
