//! Wallet authentication: nonce issuance, Sign-In-With-Solana-style
//! message verification, and session management — spec §9.1.

mod extractor;
mod handlers;
mod message;
mod session;
mod token;

pub use extractor::{AuthenticatedWallet, OptionalAuthenticatedWallet};
pub(crate) use token::generate_token;

use axum::routing::{get, post};
use axum::Router;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/auth/nonce", post(handlers::nonce))
        .route("/auth/verify", post(handlers::verify))
        .route("/auth/logout", post(handlers::logout))
        .route("/auth/me", get(handlers::me))
}
