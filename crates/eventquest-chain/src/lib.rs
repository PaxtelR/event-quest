//! Shared Solana client code for `apps/api` and `apps/indexer`, generated
//! from the `eventquest` program's on-chain IDL via Anchor's
//! `declare_program!` macro — see docs/adr/ADR-002-solana-client.md.
//!
//! This gives both services the same account layouts (`accounts::*`),
//! typed events (`AttendanceRecorded`), instruction argument encoders
//! (`client::args::*`), and instruction account-meta builders
//! (`client::accounts::*`) without hand-duplicating any of it.
//!
//! Regenerate after any program change:
//! 1. `anchor build` (regenerates `target/idl/eventquest.json`)
//! 2. `cp target/idl/eventquest.json idls/eventquest.json`
//! 3. rebuild this crate

// `declare_program!`'s off-chain code path expands to `use super::anchor_lang;`
// inside the generated module, which needs `anchor_lang` bound as a name at
// this crate's root (extern-prelude visibility alone isn't enough here).
// Load-bearing despite looking redundant to clippy — removing it breaks the
// macro expansion with `error[E0432]: unresolved import `super``.
#[allow(clippy::single_component_path_imports)]
use anchor_lang;

anchor_lang::declare_program!(eventquest);

pub use eventquest::*;

pub mod pda {
    //! PDA derivation using the exact seed bytes published by the program's
    //! IDL (`#[constant]` in `programs/eventquest/src/constants.rs`) — never
    //! hand-copy seed byte strings elsewhere.
    use anchor_lang::prelude::Pubkey;

    use super::{constants, ID};

    pub fn event(authority: &Pubkey, external_id_hash: &[u8; 32]) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[
                constants::EVENT_SEED,
                authority.as_ref(),
                external_id_hash.as_ref(),
            ],
            &ID,
        )
    }

    pub fn checkpoint(event: &Pubkey, external_id_hash: &[u8; 32]) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[
                constants::CHECKPOINT_SEED,
                event.as_ref(),
                external_id_hash.as_ref(),
            ],
            &ID,
        )
    }

    pub fn participant_event(event: &Pubkey, participant: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[
                constants::PARTICIPANT_SEED,
                event.as_ref(),
                participant.as_ref(),
            ],
            &ID,
        )
    }

    pub fn attendance(event: &Pubkey, checkpoint: &Pubkey, participant: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[
                constants::ATTENDANCE_SEED,
                event.as_ref(),
                checkpoint.as_ref(),
                participant.as_ref(),
            ],
            &ID,
        )
    }
}
