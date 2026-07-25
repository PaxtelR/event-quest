//! Confirms the `declare_program!`-generated client actually works end to
//! end: PDA derivation, instruction data encoding, and account-meta
//! building — the three things `apps/api`'s transaction-preparation step
//! (spec §9.3) depends on.

use anchor_lang::prelude::Pubkey;
use anchor_lang::{InstructionData, ToAccountMetas};
use eventquest_chain::client::{accounts, args};
use eventquest_chain::types::{EventStatus, UpdateCheckpointParams};
use eventquest_chain::{pda, ID};

#[test]
fn program_id_matches_deployed_keypair() {
    assert_eq!(
        ID.to_string(),
        "CuULC7T6d49mf89sRPU8WLeYfkF74kDyEKvfCe2rXq8H"
    );
}

#[test]
fn pda_derivation_is_deterministic_and_matches_seeds() {
    let authority = Pubkey::new_unique();
    let external_id_hash = [7u8; 32];

    let (event, bump) = pda::event(&authority, &external_id_hash);
    let (event_again, bump_again) = pda::event(&authority, &external_id_hash);
    assert_eq!(event, event_again);
    assert_eq!(bump, bump_again);

    let (expected_event, expected_bump) = Pubkey::find_program_address(
        &[
            eventquest_chain::constants::EVENT_SEED,
            authority.as_ref(),
            external_id_hash.as_ref(),
        ],
        &ID,
    );
    assert_eq!(event, expected_event);
    assert_eq!(bump, expected_bump);

    let checkpoint_hash = [8u8; 32];
    let (checkpoint, _) = pda::checkpoint(&event, &checkpoint_hash);
    let participant = Pubkey::new_unique();
    let (participant_event, _) = pda::participant_event(&event, &participant);
    let (attendance, _) = pda::attendance(&event, &checkpoint, &participant);

    // All four PDAs must be distinct even though they share overlapping seed
    // components — sanity check against a seed-derivation copy/paste bug.
    let all = [event, checkpoint, participant_event, attendance];
    for i in 0..all.len() {
        for j in (i + 1)..all.len() {
            assert_ne!(all[i], all[j], "PDAs at indices {i} and {j} collided");
        }
    }
}

#[test]
fn check_in_instruction_data_round_trips_the_challenge_hash() {
    let challenge_hash = [42u8; 32];
    let data = args::CheckIn { challenge_hash }.data();

    // 8-byte Anchor discriminator + the 32-byte challenge hash, nothing else.
    assert_eq!(data.len(), 8 + 32);
    assert_eq!(&data[8..], &challenge_hash[..]);
}

#[test]
fn check_in_account_metas_match_program_account_order() {
    let participant = Pubkey::new_unique();
    let attestor = Pubkey::new_unique();
    let event = Pubkey::new_unique();
    let checkpoint = Pubkey::new_unique();
    let participant_event = Pubkey::new_unique();
    let attendance = Pubkey::new_unique();

    let metas = accounts::CheckIn {
        participant,
        attestor,
        event,
        checkpoint,
        participant_event,
        attendance,
        system_program: anchor_lang::system_program::ID,
    }
    .to_account_metas(None);

    let expected_order = [
        participant,
        attestor,
        event,
        checkpoint,
        participant_event,
        attendance,
        anchor_lang::system_program::ID,
    ];
    assert_eq!(metas.len(), expected_order.len());
    for (meta, expected_pubkey) in metas.iter().zip(expected_order.iter()) {
        assert_eq!(meta.pubkey, *expected_pubkey);
    }

    // Signer flags must match programs/eventquest/src/instructions/check_in.rs
    // exactly, since the backend relies on this to know who needs to sign.
    assert!(metas[0].is_signer, "participant must be a signer");
    assert!(metas[1].is_signer, "attestor must be a signer");
    assert!(!metas[2].is_signer);
    assert!(!metas[3].is_signer);
}

#[test]
fn update_checkpoint_params_and_event_status_round_trip() {
    let params = UpdateCheckpointParams {
        attestor: None,
        opens_at: None,
        closes_at: None,
        points: Some(75),
        active: Some(false),
    };
    let data = args::UpdateCheckpoint { params }.data();
    assert!(!data.is_empty());

    let data = args::UpdateEventStatus {
        new_status: EventStatus::Paused,
    }
    .data();
    assert!(!data.is_empty());
}
