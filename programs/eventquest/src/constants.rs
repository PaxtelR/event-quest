use anchor_lang::prelude::*;

// `#[constant]` publishes these into the program's IDL, so off-chain clients
// (crates/eventquest-chain) can derive PDAs using the exact same seed bytes
// instead of hand-copying them — see docs/adr/ADR-002-solana-client.md.
#[constant]
pub const EVENT_SEED: &[u8] = b"event";
#[constant]
pub const CHECKPOINT_SEED: &[u8] = b"checkpoint";
#[constant]
pub const PARTICIPANT_SEED: &[u8] = b"participant";
#[constant]
pub const ATTENDANCE_SEED: &[u8] = b"attendance";

/// Upper bound on points a single checkpoint can award. Keeps
/// `ParticipantEventAccount::points` (u64, summed via checked_add) far from
/// overflow even across a large number of checkpoints, and gives the
/// organizer-facing API a sane input bound to enforce as well.
#[constant]
pub const MAX_POINTS_PER_CHECKPOINT: u32 = 10_000;
