use serde::{Deserialize, Serialize};

/// Mirrors the Postgres `checkin_attempt_status` enum
/// (apps/api/migrations/..._initial_schema.sql) and the state machine from
/// spec §9.5. `apps/api` maps this to/from the DB enum string at the
/// repository boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckinAttemptStatus {
    QrValidated,
    TransactionPrepared,
    TransactionSubmitted,
    Confirmed,
    Failed,
    Expired,
    Rejected,
}

impl CheckinAttemptStatus {
    pub fn as_db_str(self) -> &'static str {
        match self {
            Self::QrValidated => "qr_validated",
            Self::TransactionPrepared => "transaction_prepared",
            Self::TransactionSubmitted => "transaction_submitted",
            Self::Confirmed => "confirmed",
            Self::Failed => "failed",
            Self::Expired => "expired",
            Self::Rejected => "rejected",
        }
    }

    pub fn from_db_str(value: &str) -> Option<Self> {
        match value {
            "qr_validated" => Some(Self::QrValidated),
            "transaction_prepared" => Some(Self::TransactionPrepared),
            "transaction_submitted" => Some(Self::TransactionSubmitted),
            "confirmed" => Some(Self::Confirmed),
            "failed" => Some(Self::Failed),
            "expired" => Some(Self::Expired),
            "rejected" => Some(Self::Rejected),
            _ => None,
        }
    }

    /// Whether this status is a final state — once reached, the attempt
    /// never transitions again (spec §9.5's state machine has no edges
    /// leaving these).
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Confirmed | Self::Failed | Self::Expired | Self::Rejected
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn db_str_round_trips_for_every_variant() {
        let all = [
            CheckinAttemptStatus::QrValidated,
            CheckinAttemptStatus::TransactionPrepared,
            CheckinAttemptStatus::TransactionSubmitted,
            CheckinAttemptStatus::Confirmed,
            CheckinAttemptStatus::Failed,
            CheckinAttemptStatus::Expired,
            CheckinAttemptStatus::Rejected,
        ];
        for status in all {
            assert_eq!(
                CheckinAttemptStatus::from_db_str(status.as_db_str()),
                Some(status)
            );
        }
    }

    #[test]
    fn terminal_states_match_spec_state_machine() {
        assert!(!CheckinAttemptStatus::QrValidated.is_terminal());
        assert!(!CheckinAttemptStatus::TransactionPrepared.is_terminal());
        assert!(!CheckinAttemptStatus::TransactionSubmitted.is_terminal());
        assert!(CheckinAttemptStatus::Confirmed.is_terminal());
        assert!(CheckinAttemptStatus::Failed.is_terminal());
        assert!(CheckinAttemptStatus::Expired.is_terminal());
        assert!(CheckinAttemptStatus::Rejected.is_terminal());
    }
}
