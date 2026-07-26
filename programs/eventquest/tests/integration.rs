//! Integration tests for the `eventquest` Anchor program using Mollusk
//! (in-process SVM, no local validator). Requires `anchor build` (or
//! `cargo build-sbf`) to have produced `target/deploy/eventquest.so` first —
//! see spec §24 local flow (`anchor build` before `/test-rust`).
//!
//! Covers the negative-test list from
//! `EventQuest_Especificacao_IA_Solana_AI_Kit_EN_Dark.md` §25.1.

use std::collections::HashMap;

use anchor_lang::InstructionData;
use eventquest::constants::{ATTENDANCE_SEED, CHECKPOINT_SEED, EVENT_SEED, PARTICIPANT_SEED};
use eventquest::instruction as ix_data;
use eventquest::{self as program, EventStatus, UpdateCheckpointParams};
use mollusk_svm::result::{InstructionResult, ProgramResult};
use mollusk_svm::Mollusk;
use solana_account::Account;
use solana_instruction::{AccountMeta, Instruction};
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;
use solana_sdk_ids::system_program;

/// anchor-lang 1.1.2 pins `solana-pubkey ^3.0.0`; mollusk-svm 0.5.1 pins
/// `solana-pubkey ^2.2` (see programs/eventquest/Cargo.toml comment). They
/// are two structurally different `Pubkey` types at the Rust level, so
/// values crossing between "the program's" pubkeys (declare_id!, instruction
/// args, decoded account fields) and "mollusk's" pubkeys (everything passed
/// into `Mollusk`/`Instruction`/`AccountMeta`) must be converted via bytes.
fn anchor_pubkey(p: Pubkey) -> anchor_lang::prelude::Pubkey {
    anchor_lang::prelude::Pubkey::new_from_array(p.to_bytes())
}

fn anchor_program_id() -> Pubkey {
    Pubkey::new_from_array(program::ID.to_bytes())
}

/// Mollusk searches `tests/fixtures`, `BPF_OUT_DIR`/`SBF_OUT_DIR`, then the
/// current working directory for `{program_name}.so`. Point it at the
/// workspace's `target/deploy` directly so this test doesn't depend on the
/// invocation's working directory or a manually-exported env var.
fn mollusk() -> Mollusk {
    let deploy_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/deploy");
    // SAFETY: test-only, single-threaded setup before any Mollusk execution.
    unsafe {
        std::env::set_var("SBF_OUT_DIR", deploy_dir);
    }
    Mollusk::new(&anchor_program_id(), "eventquest")
}

/// Anchor custom errors are numbered from `ERROR_CODE_OFFSET` (6000) in
/// declaration order, matching `programs/eventquest/src/errors.rs`.
fn assert_custom_error(result: &InstructionResult, error: eventquest::errors::EventQuestError) {
    match &result.program_result {
        ProgramResult::Failure(ProgramError::Custom(code)) => {
            let expected = anchor_lang::error::ERROR_CODE_OFFSET + error as u32;
            assert_eq!(
                *code, expected,
                "expected EventQuestError::{error:?} ({expected}), got custom code {code}"
            );
        }
        other => panic!("expected custom error EventQuestError::{error:?}, got {other:?}"),
    }
}

fn assert_fails(result: &InstructionResult) {
    assert!(
        result.program_result.is_err(),
        "expected instruction to fail, but it succeeded"
    );
}

fn funded_account() -> Account {
    Account::new(10_000_000_000, 0, &system_program::id())
}

fn uninitialized_pda() -> Account {
    Account::default()
}

/// Minimal in-memory account store so successive `process_instruction` calls
/// see the state left behind by earlier ones, without a validator.
struct World {
    mollusk: Mollusk,
    accounts: HashMap<Pubkey, Account>,
}

impl World {
    fn new() -> Self {
        let mut accounts = HashMap::new();
        let (sys_id, sys_account) = mollusk_svm::program::keyed_account_for_system_program();
        accounts.insert(sys_id, sys_account);
        Self {
            mollusk: mollusk(),
            accounts,
        }
    }

    fn fund(&mut self, pubkey: Pubkey) {
        self.accounts.insert(pubkey, funded_account());
    }

    fn set(&mut self, pubkey: Pubkey, account: Account) {
        self.accounts.insert(pubkey, account);
    }

    fn account_for(&self, pubkey: &Pubkey) -> Account {
        self.accounts.get(pubkey).cloned().unwrap_or_default()
    }

    fn send(&mut self, instruction: &Instruction) -> InstructionResult {
        let accounts: Vec<(Pubkey, Account)> = instruction
            .accounts
            .iter()
            .map(|meta| (meta.pubkey, self.account_for(&meta.pubkey)))
            .collect();
        let result = self.mollusk.process_instruction(instruction, &accounts);
        for (pubkey, account) in &result.resulting_accounts {
            self.accounts.insert(*pubkey, account.clone());
        }
        result
    }
}

fn event_pda(authority: &Pubkey, external_id_hash: &[u8; 32]) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[EVENT_SEED, authority.as_ref(), external_id_hash.as_ref()],
        &anchor_program_id(),
    )
}

fn checkpoint_pda(event: &Pubkey, external_id_hash: &[u8; 32]) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[CHECKPOINT_SEED, event.as_ref(), external_id_hash.as_ref()],
        &anchor_program_id(),
    )
}

fn participant_pda(event: &Pubkey, participant: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[PARTICIPANT_SEED, event.as_ref(), participant.as_ref()],
        &anchor_program_id(),
    )
}

fn attendance_pda(event: &Pubkey, checkpoint: &Pubkey, participant: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            ATTENDANCE_SEED,
            event.as_ref(),
            checkpoint.as_ref(),
            participant.as_ref(),
        ],
        &anchor_program_id(),
    )
}

const HASH_A: [u8; 32] = [1u8; 32];
const HASH_B: [u8; 32] = [2u8; 32];
const CHECKPOINT_HASH: [u8; 32] = [3u8; 32];
const CHALLENGE_HASH: [u8; 32] = [9u8; 32];

const EVENT_STARTS_AT: i64 = -7_200;
const EVENT_ENDS_AT: i64 = 7_200;
const CHECKPOINT_OPENS_AT: i64 = -3_600;
const CHECKPOINT_CLOSES_AT: i64 = 3_600;
const CHECKPOINT_POINTS: u32 = 50;

fn initialize_event_ix(
    authority: Pubkey,
    event: Pubkey,
    external_id_hash: [u8; 32],
) -> Instruction {
    Instruction {
        program_id: anchor_program_id(),
        accounts: vec![
            AccountMeta::new(authority, true),
            AccountMeta::new(event, false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data: ix_data::InitializeEvent {
            external_id_hash,
            starts_at: EVENT_STARTS_AT,
            ends_at: EVENT_ENDS_AT,
        }
        .data(),
    }
}

fn create_checkpoint_ix(
    authority: Pubkey,
    event: Pubkey,
    checkpoint: Pubkey,
    external_id_hash: [u8; 32],
    attestor: Pubkey,
) -> Instruction {
    Instruction {
        program_id: anchor_program_id(),
        accounts: vec![
            AccountMeta::new(authority, true),
            AccountMeta::new(event, false),
            AccountMeta::new(checkpoint, false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data: ix_data::CreateCheckpoint {
            external_id_hash,
            attestor: anchor_pubkey(attestor),
            opens_at: CHECKPOINT_OPENS_AT,
            closes_at: CHECKPOINT_CLOSES_AT,
            points: CHECKPOINT_POINTS,
        }
        .data(),
    }
}

fn join_event_ix(participant: Pubkey, event: Pubkey, participant_event: Pubkey) -> Instruction {
    Instruction {
        program_id: anchor_program_id(),
        accounts: vec![
            AccountMeta::new(participant, true),
            AccountMeta::new_readonly(event, false),
            AccountMeta::new(participant_event, false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data: ix_data::JoinEvent {}.data(),
    }
}

#[allow(clippy::too_many_arguments)]
fn check_in_ix(
    participant: Pubkey,
    attestor: Pubkey,
    attestor_is_signer: bool,
    event: Pubkey,
    checkpoint: Pubkey,
    participant_event: Pubkey,
    attendance: Pubkey,
    challenge_hash: [u8; 32],
) -> Instruction {
    Instruction {
        program_id: anchor_program_id(),
        accounts: vec![
            AccountMeta::new(participant, true),
            AccountMeta::new_readonly(attestor, attestor_is_signer),
            AccountMeta::new(event, false),
            AccountMeta::new(checkpoint, false),
            AccountMeta::new(participant_event, false),
            AccountMeta::new(attendance, false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data: ix_data::CheckIn { challenge_hash }.data(),
    }
}

/// Fully wires up: authority + event + checkpoint (active, open window) +
/// participant joined to the event. Returns the pubkeys needed to check in.
struct Fixture {
    authority: Pubkey,
    event: Pubkey,
    checkpoint: Pubkey,
    attestor: Pubkey,
    participant: Pubkey,
    participant_event: Pubkey,
}

fn setup_fixture(world: &mut World) -> Fixture {
    let authority = Pubkey::new_unique();
    let attestor = Pubkey::new_unique();
    let participant = Pubkey::new_unique();
    world.fund(authority);
    world.fund(participant);

    let (event, _) = event_pda(&authority, &HASH_A);
    world.set(event, uninitialized_pda());
    let result = world.send(&initialize_event_ix(authority, event, HASH_A));
    assert!(result.program_result.is_ok(), "initialize_event failed");

    // Activate the event so join_event/check_in are allowed.
    let update_status_ix = Instruction {
        program_id: anchor_program_id(),
        accounts: vec![
            AccountMeta::new_readonly(authority, true),
            AccountMeta::new(event, false),
        ],
        data: ix_data::UpdateEventStatus {
            new_status: EventStatus::Active,
        }
        .data(),
    };
    let result = world.send(&update_status_ix);
    assert!(result.program_result.is_ok(), "update_event_status failed");

    let (checkpoint, _) = checkpoint_pda(&event, &CHECKPOINT_HASH);
    world.set(checkpoint, uninitialized_pda());
    let result = world.send(&create_checkpoint_ix(
        authority,
        event,
        checkpoint,
        CHECKPOINT_HASH,
        attestor,
    ));
    assert!(result.program_result.is_ok(), "create_checkpoint failed");

    let (participant_event, _) = participant_pda(&event, &participant);
    world.set(participant_event, uninitialized_pda());
    let result = world.send(&join_event_ix(participant, event, participant_event));
    assert!(result.program_result.is_ok(), "join_event failed");

    Fixture {
        authority,
        event,
        checkpoint,
        attestor,
        participant,
        participant_event,
    }
}

#[test]
fn initialize_event_succeeds_and_sets_fields() {
    let mut world = World::new();
    let authority = Pubkey::new_unique();
    world.fund(authority);
    let (event, bump) = event_pda(&authority, &HASH_A);
    world.set(event, uninitialized_pda());

    let result = world.send(&initialize_event_ix(authority, event, HASH_A));
    assert!(result.program_result.is_ok(), "{:?}", result.program_result);

    let account = result.get_account(&event).expect("event account missing");
    let data: program::EventAccount =
        anchor_lang::AccountDeserialize::try_deserialize(&mut account.data.as_slice()).unwrap();
    assert_eq!(data.authority.to_bytes(), authority.to_bytes());
    assert_eq!(data.external_id_hash, HASH_A);
    assert_eq!(data.status, EventStatus::Draft);
    assert_eq!(data.checkpoint_count, 0);
    assert_eq!(data.total_checkins, 0);
    assert_eq!(data.bump, bump);
}

#[test]
fn initialize_event_rejects_invalid_period() {
    let mut world = World::new();
    let authority = Pubkey::new_unique();
    world.fund(authority);
    let (event, _) = event_pda(&authority, &HASH_A);
    world.set(event, uninitialized_pda());

    let bad_ix = Instruction {
        program_id: anchor_program_id(),
        accounts: vec![
            AccountMeta::new(authority, true),
            AccountMeta::new(event, false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data: ix_data::InitializeEvent {
            external_id_hash: HASH_A,
            starts_at: 100,
            ends_at: 100, // not strictly after starts_at
        }
        .data(),
    };

    let result = world.send(&bad_ix);
    assert_custom_error(
        &result,
        eventquest::errors::EventQuestError::InvalidEventPeriod,
    );
}

#[test]
fn create_checkpoint_rejects_window_outside_event() {
    let mut world = World::new();
    let authority = Pubkey::new_unique();
    let attestor = Pubkey::new_unique();
    world.fund(authority);
    let (event, _) = event_pda(&authority, &HASH_A);
    world.set(event, uninitialized_pda());
    world.send(&initialize_event_ix(authority, event, HASH_A));

    let (checkpoint, _) = checkpoint_pda(&event, &CHECKPOINT_HASH);
    world.set(checkpoint, uninitialized_pda());

    let bad_ix = Instruction {
        program_id: anchor_program_id(),
        accounts: vec![
            AccountMeta::new(authority, true),
            AccountMeta::new(event, false),
            AccountMeta::new(checkpoint, false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data: ix_data::CreateCheckpoint {
            external_id_hash: CHECKPOINT_HASH,
            attestor: anchor_pubkey(attestor),
            opens_at: EVENT_STARTS_AT - 1_000, // starts before the event does
            closes_at: CHECKPOINT_CLOSES_AT,
            points: CHECKPOINT_POINTS,
        }
        .data(),
    };

    let result = world.send(&bad_ix);
    assert_custom_error(
        &result,
        eventquest::errors::EventQuestError::CheckpointOutsideEventPeriod,
    );
}

#[test]
fn create_checkpoint_rejects_unauthorized_signer() {
    let mut world = World::new();
    let authority = Pubkey::new_unique();
    let impostor = Pubkey::new_unique();
    let attestor = Pubkey::new_unique();
    world.fund(authority);
    world.fund(impostor);
    let (event, _) = event_pda(&authority, &HASH_A);
    world.set(event, uninitialized_pda());
    world.send(&initialize_event_ix(authority, event, HASH_A));

    let (checkpoint, _) = checkpoint_pda(&event, &CHECKPOINT_HASH);
    world.set(checkpoint, uninitialized_pda());

    // `impostor` signs, but `event.authority` is still the original authority.
    // The `event` account's own `seeds` constraint re-derives its PDA from
    // the *signer's* key, so this is rejected as a seeds mismatch
    // (ConstraintSeeds) before `has_one = authority` is ever evaluated —
    // an even stronger rejection than the custom Unauthorized error.
    let result = world.send(&create_checkpoint_ix(
        impostor,
        event,
        checkpoint,
        CHECKPOINT_HASH,
        attestor,
    ));
    assert_fails(&result);
}

#[test]
fn check_in_succeeds_and_awards_points_from_checkpoint() {
    let mut world = World::new();
    let f = setup_fixture(&mut world);
    world.fund(f.attestor);

    let (attendance, bump) = attendance_pda(&f.event, &f.checkpoint, &f.participant);
    world.set(attendance, uninitialized_pda());

    let result = world.send(&check_in_ix(
        f.participant,
        f.attestor,
        true,
        f.event,
        f.checkpoint,
        f.participant_event,
        attendance,
        CHALLENGE_HASH,
    ));
    assert!(result.program_result.is_ok(), "{:?}", result.program_result);

    let attendance_account = result.get_account(&attendance).unwrap();
    let data: program::AttendanceAccount =
        anchor_lang::AccountDeserialize::try_deserialize(&mut attendance_account.data.as_slice())
            .unwrap();
    assert_eq!(data.event.to_bytes(), f.event.to_bytes());
    assert_eq!(data.checkpoint.to_bytes(), f.checkpoint.to_bytes());
    assert_eq!(data.participant.to_bytes(), f.participant.to_bytes());
    assert_eq!(data.attestor.to_bytes(), f.attestor.to_bytes());
    assert_eq!(data.challenge_hash, CHALLENGE_HASH);
    assert_eq!(data.points_awarded, CHECKPOINT_POINTS);
    assert_eq!(data.bump, bump);

    let participant_event_account = result.get_account(&f.participant_event).unwrap();
    let pe: program::ParticipantEventAccount = anchor_lang::AccountDeserialize::try_deserialize(
        &mut participant_event_account.data.as_slice(),
    )
    .unwrap();
    assert_eq!(pe.points, CHECKPOINT_POINTS as u64);
    assert_eq!(pe.checkin_count, 1);
}

#[test]
fn check_in_rejects_duplicate_attempt() {
    let mut world = World::new();
    let f = setup_fixture(&mut world);
    world.fund(f.attestor);

    let (attendance, _) = attendance_pda(&f.event, &f.checkpoint, &f.participant);
    world.set(attendance, uninitialized_pda());

    let ix = check_in_ix(
        f.participant,
        f.attestor,
        true,
        f.event,
        f.checkpoint,
        f.participant_event,
        attendance,
        CHALLENGE_HASH,
    );

    let first = world.send(&ix);
    assert!(first.program_result.is_ok(), "first check-in must succeed");

    // Second attempt: the attendance PDA already exists and is owned by our
    // program, so `init` must fail — this is what makes duplicate check-ins
    // structurally impossible (spec §11.2).
    let second = world.send(&ix);
    assert_fails(&second);
}

#[test]
fn check_in_rejects_wrong_attestor() {
    let mut world = World::new();
    let f = setup_fixture(&mut world);
    let wrong_attestor = Pubkey::new_unique();
    world.fund(wrong_attestor);

    let (attendance, _) = attendance_pda(&f.event, &f.checkpoint, &f.participant);
    world.set(attendance, uninitialized_pda());

    let result = world.send(&check_in_ix(
        f.participant,
        wrong_attestor,
        true,
        f.event,
        f.checkpoint,
        f.participant_event,
        attendance,
        CHALLENGE_HASH,
    ));
    assert_custom_error(
        &result,
        eventquest::errors::EventQuestError::InvalidAttestor,
    );
}

#[test]
fn check_in_rejects_missing_attestor_signature() {
    let mut world = World::new();
    let f = setup_fixture(&mut world);
    world.fund(f.attestor);

    let (attendance, _) = attendance_pda(&f.event, &f.checkpoint, &f.participant);
    world.set(attendance, uninitialized_pda());

    // Correct attestor pubkey, but its AccountMeta claims it did not sign.
    let result = world.send(&check_in_ix(
        f.participant,
        f.attestor,
        false,
        f.event,
        f.checkpoint,
        f.participant_event,
        attendance,
        CHALLENGE_HASH,
    ));
    assert_fails(&result);
}

#[test]
fn check_in_rejects_when_event_not_active() {
    let mut world = World::new();
    let f = setup_fixture(&mut world);
    world.fund(f.attestor);

    // Pause the event after setup.
    let pause_ix = Instruction {
        program_id: anchor_program_id(),
        accounts: vec![
            AccountMeta::new_readonly(f.authority, true),
            AccountMeta::new(f.event, false),
        ],
        data: ix_data::UpdateEventStatus {
            new_status: EventStatus::Paused,
        }
        .data(),
    };
    let result = world.send(&pause_ix);
    assert!(result.program_result.is_ok(), "pause must succeed");

    let (attendance, _) = attendance_pda(&f.event, &f.checkpoint, &f.participant);
    world.set(attendance, uninitialized_pda());

    let result = world.send(&check_in_ix(
        f.participant,
        f.attestor,
        true,
        f.event,
        f.checkpoint,
        f.participant_event,
        attendance,
        CHALLENGE_HASH,
    ));
    assert_custom_error(&result, eventquest::errors::EventQuestError::EventNotActive);
}

#[test]
fn check_in_rejects_when_checkpoint_not_active() {
    let mut world = World::new();
    let f = setup_fixture(&mut world);
    world.fund(f.attestor);

    let deactivate_ix = Instruction {
        program_id: anchor_program_id(),
        accounts: vec![
            AccountMeta::new_readonly(f.authority, true),
            AccountMeta::new_readonly(f.event, false),
            AccountMeta::new(f.checkpoint, false),
        ],
        data: ix_data::UpdateCheckpoint {
            params: UpdateCheckpointParams {
                active: Some(false),
                ..Default::default()
            },
        }
        .data(),
    };
    let result = world.send(&deactivate_ix);
    assert!(result.program_result.is_ok(), "deactivate must succeed");

    let (attendance, _) = attendance_pda(&f.event, &f.checkpoint, &f.participant);
    world.set(attendance, uninitialized_pda());

    let result = world.send(&check_in_ix(
        f.participant,
        f.attestor,
        true,
        f.event,
        f.checkpoint,
        f.participant_event,
        attendance,
        CHALLENGE_HASH,
    ));
    assert_custom_error(
        &result,
        eventquest::errors::EventQuestError::CheckpointNotActive,
    );
}

#[test]
fn check_in_rejects_outside_checkpoint_window() {
    let mut world = World::new();
    let authority = Pubkey::new_unique();
    let attestor = Pubkey::new_unique();
    let participant = Pubkey::new_unique();
    world.fund(authority);
    world.fund(participant);
    world.fund(attestor);

    let (event, _) = event_pda(&authority, &HASH_A);
    world.set(event, uninitialized_pda());
    world.send(&initialize_event_ix(authority, event, HASH_A));
    world.send(&Instruction {
        program_id: anchor_program_id(),
        accounts: vec![
            AccountMeta::new_readonly(authority, true),
            AccountMeta::new(event, false),
        ],
        data: ix_data::UpdateEventStatus {
            new_status: EventStatus::Active,
        }
        .data(),
    });

    let (checkpoint, _) = checkpoint_pda(&event, &CHECKPOINT_HASH);
    world.set(checkpoint, uninitialized_pda());
    // Window is entirely in the future relative to Mollusk's default clock
    // (unix_timestamp == 0).
    let future_ix = Instruction {
        program_id: anchor_program_id(),
        accounts: vec![
            AccountMeta::new(authority, true),
            AccountMeta::new(event, false),
            AccountMeta::new(checkpoint, false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data: ix_data::CreateCheckpoint {
            external_id_hash: CHECKPOINT_HASH,
            attestor: anchor_pubkey(attestor),
            opens_at: 1_000,
            closes_at: 2_000,
            points: CHECKPOINT_POINTS,
        }
        .data(),
    };
    world.send(&future_ix);

    let (participant_event, _) = participant_pda(&event, &participant);
    world.set(participant_event, uninitialized_pda());
    world.send(&join_event_ix(participant, event, participant_event));

    let (attendance, _) = attendance_pda(&event, &checkpoint, &participant);
    world.set(attendance, uninitialized_pda());

    let result = world.send(&check_in_ix(
        participant,
        attestor,
        true,
        event,
        checkpoint,
        participant_event,
        attendance,
        CHALLENGE_HASH,
    ));
    assert_custom_error(
        &result,
        eventquest::errors::EventQuestError::CheckpointNotOpen,
    );
}

#[test]
fn check_in_rejects_checkpoint_from_another_event() {
    let mut world = World::new();
    let f = setup_fixture(&mut world);
    world.fund(f.attestor);

    // A second, unrelated event under the same authority.
    let (event_b, _) = event_pda(&f.authority, &HASH_B);
    world.set(event_b, uninitialized_pda());
    world.send(&initialize_event_ix(f.authority, event_b, HASH_B));
    world.send(&Instruction {
        program_id: anchor_program_id(),
        accounts: vec![
            AccountMeta::new_readonly(f.authority, true),
            AccountMeta::new(event_b, false),
        ],
        data: ix_data::UpdateEventStatus {
            new_status: EventStatus::Active,
        }
        .data(),
    });
    let (participant_event_b, _) = participant_pda(&event_b, &f.participant);
    world.set(participant_event_b, uninitialized_pda());
    world.send(&join_event_ix(f.participant, event_b, participant_event_b));

    let (attendance, _) = attendance_pda(&event_b, &f.checkpoint, &f.participant);
    world.set(attendance, uninitialized_pda());

    // `f.checkpoint` actually belongs to `f.event`, not `event_b`.
    let result = world.send(&check_in_ix(
        f.participant,
        f.attestor,
        true,
        event_b,
        f.checkpoint,
        participant_event_b,
        attendance,
        CHALLENGE_HASH,
    ));
    assert_fails(&result);
}

#[test]
fn check_in_rejects_on_points_overflow() {
    let mut world = World::new();
    let f = setup_fixture(&mut world);
    world.fund(f.attestor);

    // Forge the participant's accumulated points right up against u64::MAX
    // so that adding the checkpoint's points overflows `checked_add`.
    let mut pe_account = world.account_for(&f.participant_event);
    let mut pe: program::ParticipantEventAccount =
        anchor_lang::AccountDeserialize::try_deserialize(&mut pe_account.data.as_slice()).unwrap();
    pe.points = u64::MAX - 1;
    let mut data = Vec::new();
    anchor_lang::AccountSerialize::try_serialize(&pe, &mut data).unwrap();
    pe_account.data = data;
    world.set(f.participant_event, pe_account);

    let (attendance, _) = attendance_pda(&f.event, &f.checkpoint, &f.participant);
    world.set(attendance, uninitialized_pda());

    let result = world.send(&check_in_ix(
        f.participant,
        f.attestor,
        true,
        f.event,
        f.checkpoint,
        f.participant_event,
        attendance,
        CHALLENGE_HASH,
    ));
    assert_custom_error(&result, eventquest::errors::EventQuestError::Overflow);
}

#[test]
fn update_checkpoint_rejects_point_change_after_checkins() {
    let mut world = World::new();
    let f = setup_fixture(&mut world);
    world.fund(f.attestor);

    let (attendance, _) = attendance_pda(&f.event, &f.checkpoint, &f.participant);
    world.set(attendance, uninitialized_pda());
    let result = world.send(&check_in_ix(
        f.participant,
        f.attestor,
        true,
        f.event,
        f.checkpoint,
        f.participant_event,
        attendance,
        CHALLENGE_HASH,
    ));
    assert!(result.program_result.is_ok(), "check-in must succeed first");

    let update_ix = Instruction {
        program_id: anchor_program_id(),
        accounts: vec![
            AccountMeta::new_readonly(f.authority, true),
            AccountMeta::new_readonly(f.event, false),
            AccountMeta::new(f.checkpoint, false),
        ],
        data: ix_data::UpdateCheckpoint {
            params: UpdateCheckpointParams {
                points: Some(999),
                ..Default::default()
            },
        }
        .data(),
    };
    let result = world.send(&update_ix);
    assert_custom_error(
        &result,
        eventquest::errors::EventQuestError::CheckpointPointsLocked,
    );
}

#[test]
fn update_event_status_rejects_transition_from_terminal_state() {
    let mut world = World::new();
    let authority = Pubkey::new_unique();
    world.fund(authority);
    let (event, _) = event_pda(&authority, &HASH_A);
    world.set(event, uninitialized_pda());
    world.send(&initialize_event_ix(authority, event, HASH_A));

    let finish_ix = Instruction {
        program_id: anchor_program_id(),
        accounts: vec![
            AccountMeta::new_readonly(authority, true),
            AccountMeta::new(event, false),
        ],
        data: ix_data::UpdateEventStatus {
            new_status: EventStatus::Finished,
        }
        .data(),
    };
    let result = world.send(&finish_ix);
    assert!(result.program_result.is_ok(), "finishing must succeed");

    let reopen_ix = Instruction {
        program_id: anchor_program_id(),
        accounts: vec![
            AccountMeta::new_readonly(authority, true),
            AccountMeta::new(event, false),
        ],
        data: ix_data::UpdateEventStatus {
            new_status: EventStatus::Active,
        }
        .data(),
    };
    let result = world.send(&reopen_ix);
    assert_custom_error(
        &result,
        eventquest::errors::EventQuestError::InvalidStatusTransition,
    );
}

/// Not a correctness test — this exists so `/profile-cu` has a single,
/// deterministic source of truth for per-instruction CU cost instead of a
/// number quoted from memory. Run with `--nocapture` to see the table.
#[test]
fn cu_profile_prints_every_instructions_compute_units() {
    let mut world = World::new();
    let authority = Pubkey::new_unique();
    let attestor = Pubkey::new_unique();
    let participant = Pubkey::new_unique();
    world.fund(authority);
    world.fund(participant);
    world.fund(attestor);

    let mut rows: Vec<(&str, u64)> = Vec::new();
    let mut send_and_record = |world: &mut World, label: &'static str, ix: &Instruction| {
        let result = world.send(ix);
        assert!(
            result.program_result.is_ok(),
            "{label} failed: {:?}",
            result.program_result
        );
        rows.push((label, result.compute_units_consumed));
    };

    let (event, _) = event_pda(&authority, &HASH_A);
    world.set(event, uninitialized_pda());
    send_and_record(
        &mut world,
        "initialize_event",
        &initialize_event_ix(authority, event, HASH_A),
    );

    let activate_ix = Instruction {
        program_id: anchor_program_id(),
        accounts: vec![
            AccountMeta::new_readonly(authority, true),
            AccountMeta::new(event, false),
        ],
        data: ix_data::UpdateEventStatus {
            new_status: EventStatus::Active,
        }
        .data(),
    };
    send_and_record(&mut world, "update_event_status", &activate_ix);

    let (checkpoint, _) = checkpoint_pda(&event, &CHECKPOINT_HASH);
    world.set(checkpoint, uninitialized_pda());
    send_and_record(
        &mut world,
        "create_checkpoint",
        &create_checkpoint_ix(authority, event, checkpoint, CHECKPOINT_HASH, attestor),
    );

    let (participant_event, _) = participant_pda(&event, &participant);
    world.set(participant_event, uninitialized_pda());
    send_and_record(
        &mut world,
        "join_event",
        &join_event_ix(participant, event, participant_event),
    );

    let (attendance, _) = attendance_pda(&event, &checkpoint, &participant);
    world.set(attendance, uninitialized_pda());
    send_and_record(
        &mut world,
        "check_in",
        &check_in_ix(
            participant,
            attestor,
            true,
            event,
            checkpoint,
            participant_event,
            attendance,
            CHALLENGE_HASH,
        ),
    );

    let update_checkpoint_ix = Instruction {
        program_id: anchor_program_id(),
        accounts: vec![
            AccountMeta::new_readonly(authority, true),
            AccountMeta::new_readonly(event, false),
            AccountMeta::new(checkpoint, false),
        ],
        data: ix_data::UpdateCheckpoint {
            params: UpdateCheckpointParams {
                active: Some(false),
                ..Default::default()
            },
        }
        .data(),
    };
    send_and_record(&mut world, "update_checkpoint", &update_checkpoint_ix);

    let finish_participant_event_ix = Instruction {
        program_id: anchor_program_id(),
        accounts: vec![
            AccountMeta::new_readonly(authority, true),
            AccountMeta::new_readonly(event, false),
            AccountMeta::new(participant_event, false),
        ],
        data: ix_data::FinishParticipantEvent {}.data(),
    };
    send_and_record(
        &mut world,
        "finish_participant_event",
        &finish_participant_event_ix,
    );

    println!("\n=== CU profile (Mollusk, happy path) ===");
    for (label, cu) in &rows {
        println!("{label:<28} {cu:>8} CU");
    }
}
