use axum::extract::{FromRequestParts, State};
use axum::http::request::Parts;
use axum_extra::extract::CookieJar;
use eventquest_domain::ErrorCode;

use crate::error::ApiError;
use crate::state::AppState;

use super::session;

/// Extractor for handlers that require an authenticated participant.
/// Reads the session cookie, loads the session from Redis, and yields the
/// wallet address — or rejects with `AUTH_REQUIRED` (spec §20). Every
/// authenticated route (events, checkpoints, check-ins) should take this
/// as a parameter instead of re-reading cookies itself.
pub struct AuthenticatedWallet(pub String);

impl FromRequestParts<AppState> for AuthenticatedWallet {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let State(state) = State::<AppState>::from_request_parts(parts, state)
            .await
            .map_err(|_| ApiError::new(ErrorCode::InternalError))?;

        let jar = CookieJar::from_headers(&parts.headers);
        let mut redis = state.redis.clone();

        let session_id = jar
            .get(&state.config.session.cookie_name)
            .map(|cookie| cookie.value().to_string())
            .ok_or_else(|| ApiError::new(ErrorCode::AuthRequired))?;

        let session_data = session::load_session(&mut redis, &session_id)
            .await?
            .ok_or_else(|| ApiError::new(ErrorCode::AuthRequired))?;

        Ok(AuthenticatedWallet(session_data.wallet))
    }
}

/// Like `AuthenticatedWallet`, but never rejects — yields `None` when
/// there's no valid session instead of `AUTH_REQUIRED`. For endpoints that
/// work unauthenticated (public event browsing) but return richer results
/// when the caller happens to be signed in (e.g. including their own
/// organization's draft events).
pub struct OptionalAuthenticatedWallet(pub Option<String>);

impl FromRequestParts<AppState> for OptionalAuthenticatedWallet {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        match AuthenticatedWallet::from_request_parts(parts, state).await {
            Ok(AuthenticatedWallet(wallet)) => Ok(OptionalAuthenticatedWallet(Some(wallet))),
            Err(_) => Ok(OptionalAuthenticatedWallet(None)),
        }
    }
}
