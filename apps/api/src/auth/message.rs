use chrono::{DateTime, Utc};

/// A Sign-In-With-Solana-style authentication message: domain-bound,
/// nonce-bound, and time-bound, per spec §9.1. Not a full SIWS ABNF
/// implementation — this is a closed loop where the backend both renders
/// and later re-parses the exact text it issued, so a simpler line-based
/// format is sufficient and easier to audit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignInMessage {
    pub domain: String,
    pub nonce: String,
    pub issued_at: DateTime<Utc>,
    pub expiration_time: DateTime<Utc>,
}

impl SignInMessage {
    pub fn render(&self) -> String {
        format!(
            "{domain} wants you to sign in with your Solana account.\n\n\
             This request will not trigger a blockchain transaction or cost any gas fees.\n\n\
             Nonce: {nonce}\n\
             Issued At: {issued_at}\n\
             Expiration Time: {expiration}\n",
            domain = self.domain,
            nonce = self.nonce,
            issued_at = self.issued_at.to_rfc3339(),
            expiration = self.expiration_time.to_rfc3339(),
        )
    }

    /// Parses a message previously produced by `render`. Returns `None` on
    /// any malformed input — callers should treat that identically to a
    /// failed signature check (spec §20: never leak which specific
    /// validation failed).
    pub fn parse(text: &str) -> Option<Self> {
        let domain = text
            .lines()
            .next()?
            .strip_suffix(" wants you to sign in with your Solana account.")?
            .to_string();

        let mut nonce = None;
        let mut issued_at = None;
        let mut expiration_time = None;

        for line in text.lines() {
            if let Some(value) = line.strip_prefix("Nonce: ") {
                nonce = Some(value.trim().to_string());
            } else if let Some(value) = line.strip_prefix("Issued At: ") {
                issued_at = DateTime::parse_from_rfc3339(value.trim())
                    .ok()
                    .map(|dt| dt.with_timezone(&Utc));
            } else if let Some(value) = line.strip_prefix("Expiration Time: ") {
                expiration_time = DateTime::parse_from_rfc3339(value.trim())
                    .ok()
                    .map(|dt| dt.with_timezone(&Utc));
            }
        }

        Some(Self {
            domain,
            nonce: nonce?,
            issued_at: issued_at?,
            expiration_time: expiration_time?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn render_then_parse_round_trips_exactly() {
        let issued_at = Utc::now();
        let message = SignInMessage {
            domain: "eventquest.example".to_string(),
            nonce: "abc123".to_string(),
            issued_at,
            expiration_time: issued_at + Duration::seconds(300),
        };

        let rendered = message.render();
        let parsed = SignInMessage::parse(&rendered).expect("should parse a message we rendered");

        assert_eq!(parsed.domain, message.domain);
        assert_eq!(parsed.nonce, message.nonce);
        // RFC3339 round-trips to microsecond precision; compare timestamps
        // rather than exact DateTime equality to avoid sub-millisecond
        // formatting drift.
        assert_eq!(parsed.issued_at.timestamp(), message.issued_at.timestamp());
        assert_eq!(
            parsed.expiration_time.timestamp(),
            message.expiration_time.timestamp()
        );
    }

    #[test]
    fn parse_rejects_garbage() {
        assert!(SignInMessage::parse("not a real message").is_none());
        assert!(SignInMessage::parse("").is_none());
    }

    #[test]
    fn parse_rejects_missing_fields() {
        let incomplete =
            "eventquest.example wants you to sign in with your Solana account.\n\nNonce: abc\n";
        assert!(SignInMessage::parse(incomplete).is_none());
    }
}
