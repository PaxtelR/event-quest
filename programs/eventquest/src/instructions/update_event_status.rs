use anchor_lang::prelude::*;

use crate::constants::EVENT_SEED;
use crate::errors::EventQuestError;
use crate::state::{EventAccount, EventStatus};

#[derive(Accounts)]
pub struct UpdateEventStatus<'info> {
    pub authority: Signer<'info>,

    #[account(
        mut,
        has_one = authority @ EventQuestError::Unauthorized,
        seeds = [EVENT_SEED, authority.key().as_ref(), event.external_id_hash.as_ref()],
        bump = event.bump
    )]
    pub event: Account<'info, EventAccount>,
}

pub fn update_event_status_handler(
    ctx: Context<UpdateEventStatus>,
    new_status: EventStatus,
) -> Result<()> {
    let event = &mut ctx.accounts.event;
    require!(
        !event.status.is_terminal(),
        EventQuestError::InvalidStatusTransition
    );
    event.status = new_status;
    Ok(())
}
