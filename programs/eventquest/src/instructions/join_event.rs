use anchor_lang::prelude::*;

use crate::constants::{EVENT_SEED, PARTICIPANT_SEED};
use crate::errors::EventQuestError;
use crate::state::{EventAccount, EventStatus, ParticipantEventAccount};

/// Not part of the spec's minimal instruction list (§11.3), but required to
/// create `ParticipantEventAccount` safely: Anchor's `init_if_needed` is
/// banned by this project's rules (reinitialization-attack risk), so
/// `check_in` requires the account to already exist. `join_event` is the
/// one-time, plain `init` that creates it before any check-in can happen.
#[derive(Accounts)]
pub struct JoinEvent<'info> {
    #[account(mut)]
    pub participant: Signer<'info>,

    #[account(
        seeds = [EVENT_SEED, event.authority.as_ref(), event.external_id_hash.as_ref()],
        bump = event.bump
    )]
    pub event: Account<'info, EventAccount>,

    #[account(
        init,
        payer = participant,
        space = ParticipantEventAccount::DISCRIMINATOR.len() + ParticipantEventAccount::INIT_SPACE,
        seeds = [PARTICIPANT_SEED, event.key().as_ref(), participant.key().as_ref()],
        bump
    )]
    pub participant_event: Account<'info, ParticipantEventAccount>,

    pub system_program: Program<'info, System>,
}

pub fn join_event_handler(ctx: Context<JoinEvent>) -> Result<()> {
    require!(
        ctx.accounts.event.status == EventStatus::Active,
        EventQuestError::EventNotActive
    );

    let now = Clock::get()?.unix_timestamp;
    let participant_event = &mut ctx.accounts.participant_event;
    participant_event.event = ctx.accounts.event.key();
    participant_event.participant = ctx.accounts.participant.key();
    participant_event.points = 0;
    participant_event.checkin_count = 0;
    participant_event.completed = false;
    participant_event.created_at = now;
    participant_event.updated_at = now;
    participant_event.bump = ctx.bumps.participant_event;

    Ok(())
}
