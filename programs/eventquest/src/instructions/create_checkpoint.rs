use anchor_lang::prelude::*;

use crate::constants::{CHECKPOINT_SEED, EVENT_SEED, MAX_POINTS_PER_CHECKPOINT};
use crate::errors::EventQuestError;
use crate::state::{CheckpointAccount, EventAccount};

#[derive(Accounts)]
#[instruction(external_id_hash: [u8; 32])]
pub struct CreateCheckpoint<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        mut,
        has_one = authority @ EventQuestError::Unauthorized,
        seeds = [EVENT_SEED, authority.key().as_ref(), event.external_id_hash.as_ref()],
        bump = event.bump
    )]
    pub event: Account<'info, EventAccount>,

    #[account(
        init,
        payer = authority,
        space = CheckpointAccount::DISCRIMINATOR.len() + CheckpointAccount::INIT_SPACE,
        seeds = [CHECKPOINT_SEED, event.key().as_ref(), external_id_hash.as_ref()],
        bump
    )]
    pub checkpoint: Account<'info, CheckpointAccount>,

    pub system_program: Program<'info, System>,
}

pub fn create_checkpoint_handler(
    ctx: Context<CreateCheckpoint>,
    external_id_hash: [u8; 32],
    attestor: Pubkey,
    opens_at: i64,
    closes_at: i64,
    points: u32,
) -> Result<()> {
    require!(
        opens_at < closes_at,
        EventQuestError::InvalidCheckpointPeriod
    );
    require!(
        opens_at >= ctx.accounts.event.starts_at && closes_at <= ctx.accounts.event.ends_at,
        EventQuestError::CheckpointOutsideEventPeriod
    );
    require!(
        points > 0 && points <= MAX_POINTS_PER_CHECKPOINT,
        EventQuestError::PointsOutOfRange
    );

    let checkpoint = &mut ctx.accounts.checkpoint;
    checkpoint.event = ctx.accounts.event.key();
    checkpoint.attestor = attestor;
    checkpoint.external_id_hash = external_id_hash;
    checkpoint.opens_at = opens_at;
    checkpoint.closes_at = closes_at;
    checkpoint.points = points;
    checkpoint.active = true;
    checkpoint.total_checkins = 0;
    checkpoint.bump = ctx.bumps.checkpoint;

    let event = &mut ctx.accounts.event;
    event.checkpoint_count = event
        .checkpoint_count
        .checked_add(1)
        .ok_or(EventQuestError::Overflow)?;

    Ok(())
}
