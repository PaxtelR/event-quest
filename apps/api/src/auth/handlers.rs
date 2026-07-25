use axum::extract::State;
use axum::response::IntoResponse;
use axum::Json;
use axum_extra::extract::cookie::{Cookie, SameSite};
use axum_extra::extract::CookieJar;
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use eventquest_domain::ErrorCode;
use serde::{Deserialize, Serialize};

use crate::error::ApiError;
use crate::state::AppState;

use super::extractor::AuthenticatedWallet;
use super::message::SignInMessage;
use super::session;
use super::token::generate_token;

/// Auth nonces live for 5 minutes — long enough to open a wallet and sign,
/// short enough to keep the replay window tight (spec §9.1 example shows
/// the same order of magnitude).
const AUTH_NONCE_TTL_SECONDS: u64 = 300;
const AUTH_NONCE_BYTES: usize = 16; // 128 bits, matching spec §8.2's jti bar.

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NonceResponse {
    pub nonce: String,
    pub message: String,
    pub expires_at: DateTime<Utc>,
}

pub async fn nonce(State(state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    let mut redis = state.redis.clone();

    let nonce_value = generate_token(AUTH_NONCE_BYTES);
    let issued_at = Utc::now();
    let expiration_time = issued_at + ChronoDuration::seconds(AUTH_NONCE_TTL_SECONDS as i64);

    let message = SignInMessage {
        domain: app_domain(&state),
        nonce: nonce_value.clone(),
        issued_at,
        expiration_time,
    };

    session::store_nonce(&mut redis, &nonce_value, AUTH_NONCE_TTL_SECONDS).await?;

    Ok(Json(NonceResponse {
        nonce: nonce_value,
        message: message.render(),
        expires_at: expiration_time,
    }))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifyRequest {
    pub wallet: String,
    pub message: String,
    pub signature: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifyResponse {
    pub wallet: String,
}

pub async fn verify(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(body): Json<VerifyRequest>,
) -> Result<(CookieJar, Json<VerifyResponse>), ApiError> {
    let mut redis = state.redis.clone();

    let parsed = SignInMessage::parse(&body.message).ok_or_else(invalid_signature)?;

    if parsed.domain != app_domain(&state) {
        return Err(invalid_signature());
    }

    let now = Utc::now();
    if parsed.expiration_time <= parsed.issued_at || now > parsed.expiration_time {
        return Err(invalid_signature());
    }

    // Single-use: a concurrent or repeated verify with the same nonce
    // cannot both succeed.
    if !session::consume_nonce(&mut redis, &parsed.nonce).await? {
        return Err(invalid_signature());
    }

    verify_ed25519_signature(&body.wallet, &body.message, &body.signature)?;

    // The wallet controls the key, so it's safe to treat it as an
    // authenticated participant identity from here on.
    sqlx::query(
        "insert into participants (wallet_address) values ($1) \
         on conflict (wallet_address) do nothing",
    )
    .bind(&body.wallet)
    .execute(&state.db)
    .await?;

    let session_id =
        session::create_session(&mut redis, &body.wallet, state.config.session.ttl_seconds).await?;

    let cookie = build_session_cookie(&state, session_id);

    Ok((
        jar.add(cookie),
        Json(VerifyResponse {
            wallet: body.wallet,
        }),
    ))
}

pub async fn logout(State(state): State<AppState>, jar: CookieJar) -> Result<CookieJar, ApiError> {
    let mut redis = state.redis.clone();

    if let Some(cookie) = jar.get(&state.config.session.cookie_name) {
        session::delete_session(&mut redis, cookie.value()).await?;
    }

    Ok(jar.remove(Cookie::from(state.config.session.cookie_name.clone())))
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeResponse {
    pub wallet: String,
}

pub async fn me(AuthenticatedWallet(wallet): AuthenticatedWallet) -> Json<MeResponse> {
    Json(MeResponse { wallet })
}

fn invalid_signature() -> ApiError {
    ApiError::new(ErrorCode::InvalidSignature)
}

fn app_domain(state: &AppState) -> String {
    url::Url::parse(&state.config.public_app_url)
        .ok()
        .and_then(|url| url.host_str().map(str::to_string))
        .unwrap_or_else(|| state.config.public_app_url.clone())
}

fn verify_ed25519_signature(
    wallet_base58: &str,
    message: &str,
    signature_base58: &str,
) -> Result<(), ApiError> {
    let wallet_bytes: [u8; 32] = bs58::decode(wallet_base58)
        .into_vec()
        .map_err(|_| invalid_signature())?
        .try_into()
        .map_err(|_| invalid_signature())?;
    let verifying_key = VerifyingKey::from_bytes(&wallet_bytes).map_err(|_| invalid_signature())?;

    let signature_bytes: [u8; 64] = bs58::decode(signature_base58)
        .into_vec()
        .map_err(|_| invalid_signature())?
        .try_into()
        .map_err(|_| invalid_signature())?;
    let signature = Signature::from_bytes(&signature_bytes);

    verifying_key
        .verify(message.as_bytes(), &signature)
        .map_err(|_| invalid_signature())
}

fn build_session_cookie(state: &AppState, session_id: String) -> Cookie<'static> {
    Cookie::build((state.config.session.cookie_name.clone(), session_id))
        .http_only(true)
        .secure(true)
        .same_site(SameSite::Lax)
        .path("/")
        .max_age(time::Duration::seconds(
            state.config.session.ttl_seconds as i64,
        ))
        .build()
}
