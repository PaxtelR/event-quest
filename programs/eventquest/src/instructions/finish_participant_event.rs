use anchor_lang::prelude::*;

use crate::constants::{EVENT_SEED, PARTICIPANT_SEED};
use crate::errors::EventQuestError;
use crate::state::{EventAccount, ParticipantEventAccount};

/// Optional in the MVP per spec §11.3. Kept intentionally minimal: the
/// backend decides *when* completion criteria (missions) are met and calls
/// this as the event authority; the program only records the boolean flag
/// and refuses to flip it twice.
#[derive(Accounts)]
pub struct FinishParticipantEvent<'info> {
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
        seeds = [PARTICIPANT_SEED, event.key().as_ref(), participant_event.participant.as_ref()],
        bump = participant_event.bump
    )]
    pub participant_event: Account<'info, ParticipantEventAccount>,
}

pub fn finish_participant_event_handler(ctx: Context<FinishParticipantEvent>) -> Result<()> {
    let participant_event = &mut ctx.accounts.participant_event;
    require!(
        !participant_event.completed,
        EventQuestError::InvalidStatusTransition
    );
    participant_event.completed = true;
    participant_event.updated_at = Clock::get()?.unix_timestamp;
    Ok(())
}
