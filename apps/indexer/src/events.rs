//! Decodes `AttendanceRecorded` events out of a confirmed transaction's
//! program logs — spec §16 step 3. Anchor's `emit!` (the non-CPI event
//! path this program uses) logs each event as `Program data: <base64>`
//! via `sol_log_data`, where the decoded bytes are the event's
//! discriminator followed by its Borsh-serialized fields (see
//! `anchor_lang::Event`/`Discriminator`) — there is no on-chain IDL
//! decoder to call into off-chain, so this reimplements exactly that
//! framing.

use anchor_lang::{AnchorDeserialize, Discriminator};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use eventquest_chain::events::AttendanceRecorded;

const LOG_PREFIX: &str = "Program data: ";

/// Scans every `Program data:` log line for one matching
/// `AttendanceRecorded`'s discriminator. A transaction only ever contains
/// one `check_in` instruction in this program's design, so the first
/// match is returned; other event types (there are none yet) would simply
/// fail the discriminator check and be skipped.
pub fn find_attendance_recorded(log_messages: &[String]) -> Option<AttendanceRecorded> {
    for line in log_messages {
        let Some(encoded) = line.strip_prefix(LOG_PREFIX) else {
            continue;
        };
        let Ok(bytes) = BASE64.decode(encoded) else {
            continue;
        };
        let discriminator = AttendanceRecorded::DISCRIMINATOR;
        if bytes.len() < discriminator.len() || &bytes[..discriminator.len()] != discriminator {
            continue;
        }
        if let Ok(event) = AttendanceRecorded::try_from_slice(&bytes[discriminator.len()..]) {
            return Some(event);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use anchor_lang::prelude::Pubkey;
    use anchor_lang::AnchorSerialize;

    fn encode_event(event: &AttendanceRecorded) -> String {
        let mut bytes = AttendanceRecorded::DISCRIMINATOR.to_vec();
        event.serialize(&mut bytes).unwrap();
        format!("{LOG_PREFIX}{}", BASE64.encode(bytes))
    }

    #[test]
    fn finds_and_decodes_a_matching_log_line_among_others() {
        let event = AttendanceRecorded {
            event: Pubkey::new_unique(),
            checkpoint: Pubkey::new_unique(),
            participant: Pubkey::new_unique(),
            attestor: Pubkey::new_unique(),
            checked_in_at: 1_700_000_000,
            points_awarded: 50,
            challenge_hash: [7u8; 32],
        };
        let logs = vec![
            "Program 11111111111111111111111111111111 invoke [1]".to_string(),
            "Program log: some other log line".to_string(),
            encode_event(&event),
            "Program 11111111111111111111111111111111 success".to_string(),
        ];

        let decoded = find_attendance_recorded(&logs).expect("event should be found");
        assert_eq!(decoded.event, event.event);
        assert_eq!(decoded.checkpoint, event.checkpoint);
        assert_eq!(decoded.participant, event.participant);
        assert_eq!(decoded.attestor, event.attestor);
        assert_eq!(decoded.points_awarded, 50);
        assert_eq!(decoded.challenge_hash, [7u8; 32]);
    }

    #[test]
    fn ignores_logs_with_a_different_discriminator() {
        let mut bytes = [0xffu8; 8].to_vec();
        bytes.extend_from_slice(&[1, 2, 3]);
        let logs = vec![format!("{LOG_PREFIX}{}", BASE64.encode(bytes))];
        assert!(find_attendance_recorded(&logs).is_none());
    }

    #[test]
    fn ignores_non_program_data_lines() {
        let logs = vec!["Program log: nothing to see here".to_string()];
        assert!(find_attendance_recorded(&logs).is_none());
    }
}
