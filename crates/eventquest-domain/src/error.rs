use http::StatusCode;
use serde::Serialize;
use uuid::Uuid;

/// The standardized error codes from spec §20. Every non-2xx API response
/// uses one of these — never an ad hoc string — so the frontend can branch
/// on `code` reliably instead of parsing `message`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    AuthRequired,
    WalletMismatch,
    InvalidSignature,
    QrInvalid,
    QrExpired,
    QrRevoked,
    EventNotActive,
    CheckpointNotActive,
    CheckpointNotOpen,
    ParticipantNotRegistered,
    AlreadyCheckedIn,
    CheckinPending,
    GrantExpired,
    TransactionExpired,
    TransactionRejected,
    TransactionFailed,
    NetworkMismatch,
    ChainUnavailable,
    RateLimited,
    InternalError,

    // --- Pragmatic extensions beyond spec §20's literal list -----------
    // §20's codes are scoped tightly to the check-in flow; generic CRUD
    // endpoints (events, checkpoints, organizations) still need a way to
    // report "doesn't exist", "you can't do that", and "bad input" without
    // inventing ad hoc strings per handler. Kept separate from the spec
    // list above so it's obvious which codes are contractually mandated
    // and which are this implementation's own additions.
    ResourceNotFound,
    Forbidden,
    ValidationError,
}

impl ErrorCode {
    /// Default English message for this code. Call sites may pass a more
    /// specific message where useful, but this covers the common case and
    /// guarantees every code always has *some* user-facing text — spec §20
    /// requires messages that are clear without exposing internal details.
    pub fn default_message(self) -> &'static str {
        match self {
            Self::AuthRequired => "You need to connect and authenticate your wallet first.",
            Self::WalletMismatch => "This action does not match the connected wallet.",
            Self::InvalidSignature => "The wallet signature could not be verified.",
            Self::QrInvalid => "This QR code is not valid.",
            Self::QrExpired => "This QR code has expired. Scan the current code.",
            Self::QrRevoked => "This QR code has been revoked.",
            Self::EventNotActive => "This event is not currently active.",
            Self::CheckpointNotActive => "This checkpoint is not currently active.",
            Self::CheckpointNotOpen => "This checkpoint is not open at this time.",
            Self::ParticipantNotRegistered => "You are not registered for this event.",
            Self::AlreadyCheckedIn => "You have already checked in at this checkpoint.",
            Self::CheckinPending => "A check-in for this checkpoint is already in progress.",
            Self::GrantExpired => {
                "This check-in authorization has expired. Scan the QR code again."
            }
            Self::TransactionExpired => "The transaction expired before it could be confirmed.",
            Self::TransactionRejected => "The transaction was rejected.",
            Self::TransactionFailed => "The transaction failed. Please try again.",
            Self::NetworkMismatch => "Your wallet is not connected to Solana Devnet.",
            Self::ChainUnavailable => "The Solana network is temporarily unavailable.",
            Self::RateLimited => "Too many requests. Please wait and try again.",
            Self::InternalError => "Something went wrong. Please try again.",
            Self::ResourceNotFound => "The requested resource was not found.",
            Self::Forbidden => "You don't have permission to do that.",
            Self::ValidationError => "The request contains invalid data.",
        }
    }

    /// HTTP status this error code is reported under. Domain-level only
    /// (the `http` crate, not axum) so this stays usable from any service.
    pub fn http_status(self) -> StatusCode {
        match self {
            Self::AuthRequired => StatusCode::UNAUTHORIZED,
            Self::WalletMismatch | Self::InvalidSignature => StatusCode::FORBIDDEN,
            Self::QrInvalid
            | Self::QrExpired
            | Self::QrRevoked
            | Self::EventNotActive
            | Self::CheckpointNotActive
            | Self::CheckpointNotOpen
            | Self::ParticipantNotRegistered
            | Self::GrantExpired
            | Self::TransactionExpired
            | Self::TransactionRejected
            | Self::NetworkMismatch => StatusCode::BAD_REQUEST,
            Self::AlreadyCheckedIn | Self::CheckinPending => StatusCode::CONFLICT,
            Self::TransactionFailed | Self::ChainUnavailable => StatusCode::SERVICE_UNAVAILABLE,
            Self::RateLimited => StatusCode::TOO_MANY_REQUESTS,
            Self::InternalError => StatusCode::INTERNAL_SERVER_ERROR,
            Self::ResourceNotFound => StatusCode::NOT_FOUND,
            Self::Forbidden => StatusCode::FORBIDDEN,
            Self::ValidationError => StatusCode::BAD_REQUEST,
        }
    }
}

/// Wire format for every error response, per spec §20:
/// `{ "code": "...", "message": "...", "requestId": "..." }`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiErrorBody {
    pub code: ErrorCode,
    pub message: String,
    pub request_id: Uuid,
}

impl ApiErrorBody {
    pub fn new(code: ErrorCode, request_id: Uuid) -> Self {
        Self {
            code,
            message: code.default_message().to_string(),
            request_id,
        }
    }

    pub fn with_message(code: ErrorCode, request_id: Uuid, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            request_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_codes_serialize_to_spec_strings() {
        let cases = [
            (ErrorCode::AuthRequired, "\"AUTH_REQUIRED\""),
            (ErrorCode::WalletMismatch, "\"WALLET_MISMATCH\""),
            (ErrorCode::InvalidSignature, "\"INVALID_SIGNATURE\""),
            (ErrorCode::QrInvalid, "\"QR_INVALID\""),
            (ErrorCode::QrExpired, "\"QR_EXPIRED\""),
            (ErrorCode::QrRevoked, "\"QR_REVOKED\""),
            (ErrorCode::EventNotActive, "\"EVENT_NOT_ACTIVE\""),
            (ErrorCode::CheckpointNotActive, "\"CHECKPOINT_NOT_ACTIVE\""),
            (ErrorCode::CheckpointNotOpen, "\"CHECKPOINT_NOT_OPEN\""),
            (
                ErrorCode::ParticipantNotRegistered,
                "\"PARTICIPANT_NOT_REGISTERED\"",
            ),
            (ErrorCode::AlreadyCheckedIn, "\"ALREADY_CHECKED_IN\""),
            (ErrorCode::CheckinPending, "\"CHECKIN_PENDING\""),
            (ErrorCode::GrantExpired, "\"GRANT_EXPIRED\""),
            (ErrorCode::TransactionExpired, "\"TRANSACTION_EXPIRED\""),
            (ErrorCode::TransactionRejected, "\"TRANSACTION_REJECTED\""),
            (ErrorCode::TransactionFailed, "\"TRANSACTION_FAILED\""),
            (ErrorCode::NetworkMismatch, "\"NETWORK_MISMATCH\""),
            (ErrorCode::ChainUnavailable, "\"CHAIN_UNAVAILABLE\""),
            (ErrorCode::RateLimited, "\"RATE_LIMITED\""),
            (ErrorCode::InternalError, "\"INTERNAL_ERROR\""),
            (ErrorCode::ResourceNotFound, "\"RESOURCE_NOT_FOUND\""),
            (ErrorCode::Forbidden, "\"FORBIDDEN\""),
            (ErrorCode::ValidationError, "\"VALIDATION_ERROR\""),
        ];

        for (code, expected) in cases {
            assert_eq!(serde_json::to_string(&code).unwrap(), expected);
        }
    }

    #[test]
    fn error_body_serializes_with_camel_case_request_id() {
        let id = Uuid::nil();
        let body = ApiErrorBody::new(ErrorCode::QrExpired, id);
        let json = serde_json::to_value(&body).unwrap();
        assert_eq!(json["code"], "QR_EXPIRED");
        assert_eq!(json["requestId"], id.to_string());
        assert_eq!(json["message"], ErrorCode::QrExpired.default_message());
    }
}
