pub const EVENT_SEED: &[u8] = b"event";
pub const CHECKPOINT_SEED: &[u8] = b"checkpoint";
pub const PARTICIPANT_SEED: &[u8] = b"participant";
pub const ATTENDANCE_SEED: &[u8] = b"attendance";

/// Upper bound on points a single checkpoint can award. Keeps
/// `ParticipantEventAccount::points` (u64, summed via checked_add) far from
/// overflow even across a large number of checkpoints, and gives the
/// organizer-facing API a sane input bound to enforce as well.
pub const MAX_POINTS_PER_CHECKPOINT: u32 = 10_000;
