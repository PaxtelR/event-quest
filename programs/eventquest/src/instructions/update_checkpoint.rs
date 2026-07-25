use anchor_lang::prelude::*;

use crate::constants::{CHECKPOINT_SEED, EVENT_SEED, MAX_POINTS_PER_CHECKPOINT};
use crate::errors::EventQuestError;
use crate::state::{CheckpointAccount, EventAccount};

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default)]
pub struct UpdateCheckpointParams {
    pub attestor: Option<Pubkey>,
    pub opens_at: Option<i64>,
    pub closes_at: Option<i64>,
    pub points: Option<u32>,
    pub active: Option<bool>,
}

#[derive(Accounts)]
pub struct UpdateCheckpoint<'info> {
    pub authority: Signer<'info>,

    #[account(
        has_one = authority @ EventQuestError::Unauthorized,
        seeds = [EVENT_SEED, authority.key().as_ref(), event.external_id_hash.as_ref()],
        bump = event.bump
    )]
    pub event: Account<'info, EventAccount>,

    #[account(
        mut,
        has_one = event @ EventQuestError::EventMismatch,
        seeds = [CHECKPOINT_SEED, event.key().as_ref(), checkpoint.external_id_hash.as_ref()],
        bump = checkpoint.bump
    )]
    pub checkpoint: Account<'info, CheckpointAccount>,
}

pub fn update_checkpoint_handler(
    ctx: Context<UpdateCheckpoint>,
    params: UpdateCheckpointParams,
) -> Result<()> {
    let event_starts_at = ctx.accounts.event.starts_at;
    let event_ends_at = ctx.accounts.event.ends_at;
    let checkpoint = &mut ctx.accounts.checkpoint;

    if let Some(points) = params.points {
        // Points already awarded to participants must never change
        // retroactively — see spec §11.3 "update_checkpoint".
        require!(
            checkpoint.total_checkins == 0,
            EventQuestError::CheckpointPointsLocked
        );
        require!(
            points > 0 && points <= MAX_POINTS_PER_CHECKPOINT,
            EventQuestError::PointsOutOfRange
        );
        checkpoint.points = points;
    }

    if let Some(attestor) = params.attestor {
        checkpoint.attestor = attestor;
    }

    let opens_at = params.opens_at.unwrap_or(checkpoint.opens_at);
    let closes_at = params.closes_at.unwrap_or(checkpoint.closes_at);
    require!(
        opens_at < closes_at,
        EventQuestError::InvalidCheckpointPeriod
    );
    require!(
        opens_at >= event_starts_at && closes_at <= event_ends_at,
        EventQuestError::CheckpointOutsideEventPeriod
    );
    checkpoint.opens_at = opens_at;
    checkpoint.closes_at = closes_at;

    if let Some(active) = params.active {
        checkpoint.active = active;
    }

    Ok(())
}
