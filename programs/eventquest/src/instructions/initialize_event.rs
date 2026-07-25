use anchor_lang::prelude::*;

use crate::constants::EVENT_SEED;
use crate::errors::EventQuestError;
use crate::state::{EventAccount, EventStatus};

#[derive(Accounts)]
#[instruction(external_id_hash: [u8; 32])]
pub struct InitializeEvent<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        init,
        payer = authority,
        space = EventAccount::DISCRIMINATOR.len() + EventAccount::INIT_SPACE,
        seeds = [EVENT_SEED, authority.key().as_ref(), external_id_hash.as_ref()],
        bump
    )]
    pub event: Account<'info, EventAccount>,

    pub system_program: Program<'info, System>,
}

pub fn initialize_event_handler(
    ctx: Context<InitializeEvent>,
    external_id_hash: [u8; 32],
    starts_at: i64,
    ends_at: i64,
) -> Result<()> {
    require!(starts_at < ends_at, EventQuestError::InvalidEventPeriod);

    let event = &mut ctx.accounts.event;
    event.authority = ctx.accounts.authority.key();
    event.external_id_hash = external_id_hash;
    event.starts_at = starts_at;
    event.ends_at = ends_at;
    event.status = EventStatus::Draft;
    event.checkpoint_count = 0;
    event.total_checkins = 0;
    event.bump = ctx.bumps.event;

    Ok(())
}
