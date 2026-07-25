use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use uuid::Uuid;

pub const QR_SCOPE: &str = "eventquest:attendance";
pub const QR_VERSION: u8 = 1;
pub const DISPLAY_SCOPE: &str = "eventquest:display";

/// The rotating QR token payload — spec §8.2. Signed HS256 (spec §8.3's
/// MVP recommendation), never contains a prepared transaction or a secret.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QrClaims {
    pub v: u8,
    pub scope: String,
    #[serde(rename = "eventId")]
    pub event_id: Uuid,
    #[serde(rename = "checkpointId")]
    pub checkpoint_id: Uuid,
    pub jti: String,
    pub iat: i64,
    pub exp: i64,
}

/// A short/medium-duration token gating the public QR display routes
/// (spec §8.4: "deve exigir um displayAccessToken... para evitar que
/// qualquer pessoa descubra checkpoints privados"). Issued only to
/// authenticated organization members via
/// `POST /checkpoints/{id}/display-token`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayClaims {
    pub scope: String,
    #[serde(rename = "checkpointId")]
    pub checkpoint_id: Uuid,
    pub iat: i64,
    pub exp: i64,
}

pub fn sign<T: Serialize>(claims: &T, secret: &str) -> Result<String, jsonwebtoken::errors::Error> {
    encode(
        &Header::new(Algorithm::HS256),
        claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
}

/// Decodes and verifies a token, tolerating `leeway_seconds` of clock skew
/// around `exp` — used for the QR token's grace window (spec §8.1
/// `QR_GRACE_SECONDS`) and given a `0` leeway for the stricter display
/// token.
pub fn verify<T: DeserializeOwned>(
    token: &str,
    secret: &str,
    leeway_seconds: u64,
) -> Result<T, jsonwebtoken::errors::Error> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.leeway = leeway_seconds;
    validation.required_spec_claims.clear();
    validation.validate_exp = true;
    let data = decode::<T>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )?;
    Ok(data.claims)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qr_claims_round_trip() {
        let now = chrono::Utc::now().timestamp();
        let claims = QrClaims {
            v: QR_VERSION,
            scope: QR_SCOPE.to_string(),
            event_id: Uuid::new_v4(),
            checkpoint_id: Uuid::new_v4(),
            jti: "abc123".to_string(),
            iat: now,
            exp: now + 15,
        };

        let token = sign(&claims, "test-secret").unwrap();
        let decoded: QrClaims = verify(&token, "test-secret", 0).unwrap();

        assert_eq!(decoded.event_id, claims.event_id);
        assert_eq!(decoded.checkpoint_id, claims.checkpoint_id);
        assert_eq!(decoded.jti, claims.jti);
    }

    #[test]
    fn verify_rejects_wrong_secret() {
        let claims = QrClaims {
            v: QR_VERSION,
            scope: QR_SCOPE.to_string(),
            event_id: Uuid::new_v4(),
            checkpoint_id: Uuid::new_v4(),
            jti: "abc123".to_string(),
            iat: 1_000,
            exp: 1_015,
        };
        let token = sign(&claims, "right-secret").unwrap();
        assert!(verify::<QrClaims>(&token, "wrong-secret", 0).is_err());
    }

    #[test]
    fn verify_rejects_expired_token_outside_leeway() {
        use chrono::Utc;
        let now = Utc::now().timestamp();
        let claims = QrClaims {
            v: QR_VERSION,
            scope: QR_SCOPE.to_string(),
            event_id: Uuid::new_v4(),
            checkpoint_id: Uuid::new_v4(),
            jti: "abc123".to_string(),
            iat: now - 100,
            exp: now - 50,
        };
        let token = sign(&claims, "secret").unwrap();

        // No leeway: an expiry 50s in the past must fail.
        assert!(verify::<QrClaims>(&token, "secret", 0).is_err());
        // Grace window covering the gap must succeed.
        assert!(verify::<QrClaims>(&token, "secret", 60).is_ok());
    }
}
