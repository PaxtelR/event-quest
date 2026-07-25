use anchor_lang::prelude::*;

use crate::constants::{ATTENDANCE_SEED, CHECKPOINT_SEED, EVENT_SEED, PARTICIPANT_SEED};
use crate::errors::EventQuestError;
use crate::events::AttendanceRecorded;
use crate::state::{
    AttendanceAccount, CheckpointAccount, EventAccount, EventStatus, ParticipantEventAccount,
};

#[derive(Accounts)]
pub struct CheckIn<'info> {
    #[account(mut)]
    pub participant: Signer<'info>,

    /// Backend attestor for this checkpoint. Must match `checkpoint.attestor`
    /// — the program never trusts a JWT or Redis state, only this signature.
    pub attestor: Signer<'info>,

    #[account(
        mut,
        seeds = [EVENT_SEED, event.authority.as_ref(), event.external_id_hash.as_ref()],
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

    #[account(
        mut,
        has_one = event @ EventQuestError::EventMismatch,
        has_one = participant @ EventQuestError::Unauthorized,
        seeds = [PARTICIPANT_SEED, event.key().as_ref(), participant.key().as_ref()],
        bump = participant_event.bump
    )]
    pub participant_event: Account<'info, ParticipantEventAccount>,

    // `init` (never `init_if_needed`) on seeds unique to
    // (event, checkpoint, participant) is what makes duplicate check-ins
    // impossible: a second attempt fails here with "account already in use"
    // before any handler logic runs. See spec §11.2.
    #[account(
        init,
        payer = participant,
        space = AttendanceAccount::DISCRIMINATOR.len() + AttendanceAccount::INIT_SPACE,
        seeds = [
            ATTENDANCE_SEED,
            event.key().as_ref(),
            checkpoint.key().as_ref(),
            participant.key().as_ref()
        ],
        bump
    )]
    pub attendance: Account<'info, AttendanceAccount>,

    pub system_program: Program<'info, System>,
}

pub fn check_in_handler(ctx: Context<CheckIn>, challenge_hash: [u8; 32]) -> Result<()> {
    require!(
        challenge_hash != [0u8; 32],
        EventQuestError::InvalidChallengeHash
    );
    require!(
        ctx.accounts.event.status == EventStatus::Active,
        EventQuestError::EventNotActive
    );
    require!(
        ctx.accounts.checkpoint.active,
        EventQuestError::CheckpointNotActive
    );
    require_keys_eq!(
        ctx.accounts.attestor.key(),
        ctx.accounts.checkpoint.attestor,
        EventQuestError::InvalidAttestor
    );

    let now = Clock::get()?.unix_timestamp;
    require!(
        now >= ctx.accounts.checkpoint.opens_at && now <= ctx.accounts.checkpoint.closes_at,
        EventQuestError::CheckpointNotOpen
    );

    // Points always come from the checkpoint, never from the client —
    // spec §11.5 "Não confiar em valores de pontos enviados pelo frontend."
    let points = ctx.accounts.checkpoint.points;
    let event_key = ctx.accounts.event.key();
    let checkpoint_key = ctx.accounts.checkpoint.key();
    let participant_key = ctx.accounts.participant.key();
    let attestor_key = ctx.accounts.attestor.key();

    let attendance = &mut ctx.accounts.attendance;
    attendance.event = event_key;
    attendance.checkpoint = checkpoint_key;
    attendance.participant = participant_key;
    attendance.attestor = attestor_key;
    attendance.challenge_hash = challenge_hash;
    attendance.checked_in_at = now;
    attendance.points_awarded = points;
    attendance.bump = ctx.bumps.attendance;

    let participant_event = &mut ctx.accounts.participant_event;
    participant_event.points = participant_event
        .points
        .checked_add(points as u64)
        .ok_or(EventQuestError::Overflow)?;
    participant_event.checkin_count = participant_event
        .checkin_count
        .checked_add(1)
        .ok_or(EventQuestError::Overflow)?;
    participant_event.updated_at = now;

    ctx.accounts.checkpoint.total_checkins = ctx
        .accounts
        .checkpoint
        .total_checkins
        .checked_add(1)
        .ok_or(EventQuestError::Overflow)?;

    ctx.accounts.event.total_checkins = ctx
        .accounts
        .event
        .total_checkins
        .checked_add(1)
        .ok_or(EventQuestError::Overflow)?;

    emit!(AttendanceRecorded {
        event: event_key,
        checkpoint: checkpoint_key,
        participant: participant_key,
        attestor: attestor_key,
        checked_in_at: now,
        points_awarded: points,
        challenge_hash,
    });

    Ok(())
}
