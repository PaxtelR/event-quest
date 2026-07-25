use anchor_lang::prelude::*;

pub mod constants;
pub mod errors;
pub mod events;
pub mod instructions;
pub mod state;

pub use instructions::*;
pub use state::*;

declare_id!("CuULC7T6d49mf89sRPU8WLeYfkF74kDyEKvfCe2rXq8H");

#[program]
pub mod eventquest {
    use super::*;

    pub fn initialize_event(
        ctx: Context<InitializeEvent>,
        external_id_hash: [u8; 32],
        starts_at: i64,
        ends_at: i64,
    ) -> Result<()> {
        initialize_event_handler(ctx, external_id_hash, starts_at, ends_at)
    }

    pub fn update_event_status(
        ctx: Context<UpdateEventStatus>,
        new_status: EventStatus,
    ) -> Result<()> {
        update_event_status_handler(ctx, new_status)
    }

    pub fn create_checkpoint(
        ctx: Context<CreateCheckpoint>,
        external_id_hash: [u8; 32],
        attestor: Pubkey,
        opens_at: i64,
        closes_at: i64,
        points: u32,
    ) -> Result<()> {
        create_checkpoint_handler(ctx, external_id_hash, attestor, opens_at, closes_at, points)
    }

    pub fn update_checkpoint(
        ctx: Context<UpdateCheckpoint>,
        params: UpdateCheckpointParams,
    ) -> Result<()> {
        update_checkpoint_handler(ctx, params)
    }

    pub fn join_event(ctx: Context<JoinEvent>) -> Result<()> {
        join_event_handler(ctx)
    }

    pub fn check_in(ctx: Context<CheckIn>, challenge_hash: [u8; 32]) -> Result<()> {
        check_in_handler(ctx, challenge_hash)
    }

    pub fn finish_participant_event(ctx: Context<FinishParticipantEvent>) -> Result<()> {
        finish_participant_event_handler(ctx)
    }
}
