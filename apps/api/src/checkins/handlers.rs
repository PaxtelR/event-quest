use anchor_lang::{InstructionData, ToAccountMetas};
use axum::extract::{Path, State};
use axum::Json;
use chrono::Utc;
use eventquest_chain::client::{accounts, args};
use eventquest_chain::pda;
use eventquest_domain::ErrorCode;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::AuthenticatedWallet;
use crate::blockchain::parse_pubkey;
use crate::error::ApiError;
use crate::qr;
use crate::state::AppState;

use super::grant::{self, CheckinGrant};
use super::models::{
    CheckinStatusResponse, GrantResponse, PrepareTransactionResponse, SubmittedRequest,
    ValidateRequest,
};

/// Simple fixed-window limiter: at most `limit` calls per `window_seconds`
/// per wallet — spec §9.2 validation #17. Not shared with any other
/// endpoint, so a bare Redis `INCR`/`EXPIRE` is enough; no need for a
/// general-purpose rate-limit crate for this one call site.
async fn check_rate_limit(
    state: &AppState,
    wallet: &str,
    limit: u64,
    window_seconds: u64,
) -> Result<(), ApiError> {
    let mut redis = state.redis.clone();
    let key = format!("eventquest:ratelimit:checkin-validate:{wallet}");
    let count: u64 = redis::cmd("INCR").arg(&key).query_async(&mut redis).await?;
    if count == 1 {
        let _: () = redis::cmd("EXPIRE")
            .arg(&key)
            .arg(window_seconds)
            .query_async(&mut redis)
            .await?;
    }
    if count > limit {
        return Err(ApiError::new(ErrorCode::RateLimited));
    }
    Ok(())
}

async fn get_or_create_participant(db: &PgPool, wallet: &str) -> Result<Uuid, ApiError> {
    if let Some(id) =
        sqlx::query_scalar::<_, Uuid>("select id from participants where wallet_address = $1")
            .bind(wallet)
            .fetch_optional(db)
            .await?
    {
        return Ok(id);
    }

    let id: Uuid = sqlx::query_scalar(
        "insert into participants (wallet_address) values ($1) \
         on conflict (wallet_address) do update set wallet_address = excluded.wallet_address \
         returning id",
    )
    .bind(wallet)
    .fetch_one(db)
    .await?;
    Ok(id)
}

#[derive(sqlx::FromRow)]
struct EventRow {
    status: String,
    onchain_event_address: Option<String>,
}

#[derive(sqlx::FromRow)]
struct CheckpointRow {
    event_id: Uuid,
    status: String,
    opens_at: chrono::DateTime<Utc>,
    closes_at: chrono::DateTime<Utc>,
    attestor_pubkey: String,
    onchain_checkpoint_address: Option<String>,
}

async fn load_event(db: &PgPool, event_id: Uuid) -> Result<EventRow, ApiError> {
    sqlx::query_as::<_, EventRow>(
        "select status::text, onchain_event_address from events where id = $1",
    )
    .bind(event_id)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| ApiError::new(ErrorCode::ResourceNotFound))
}

async fn load_checkpoint(db: &PgPool, checkpoint_id: Uuid) -> Result<CheckpointRow, ApiError> {
    sqlx::query_as::<_, CheckpointRow>(
        "select event_id, status::text, opens_at, closes_at, attestor_pubkey, \
         onchain_checkpoint_address from checkpoints where id = $1",
    )
    .bind(checkpoint_id)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| ApiError::new(ErrorCode::ResourceNotFound))
}

/// spec §9.2: `POST /check-ins/validate`. Runs the full 17-point
/// validation list and, on success, creates the individual authorization
/// ("grant") that `prepare-transaction` consumes next.
pub async fn validate(
    State(state): State<AppState>,
    AuthenticatedWallet(session_wallet): AuthenticatedWallet,
    Json(body): Json<ValidateRequest>,
) -> Result<Json<GrantResponse>, ApiError> {
    if body.wallet != session_wallet {
        return Err(ApiError::new(ErrorCode::WalletMismatch));
    }
    let wallet = session_wallet;

    check_rate_limit(&state, &wallet, 10, 60).await?;

    let claims: qr::QrClaims = qr::verify(&body.qr_token, &state.config.qr.signing_secret, 0)
        .map_err(|error| {
            if matches!(
                error.kind(),
                jsonwebtoken::errors::ErrorKind::ExpiredSignature
            ) {
                ApiError::new(ErrorCode::QrExpired)
            } else {
                ApiError::new(ErrorCode::QrInvalid)
            }
        })?;
    if claims.scope != qr::QR_SCOPE || claims.v != qr::QR_VERSION {
        return Err(ApiError::new(ErrorCode::QrInvalid));
    }

    let event = load_event(&state.db, claims.event_id).await?;
    let checkpoint = load_checkpoint(&state.db, claims.checkpoint_id).await?;
    if checkpoint.event_id != claims.event_id {
        return Err(ApiError::new(ErrorCode::QrInvalid));
    }
    if event.status != "active" {
        return Err(ApiError::new(ErrorCode::EventNotActive));
    }
    if checkpoint.status != "active" {
        return Err(ApiError::new(ErrorCode::CheckpointNotActive));
    }
    let now = Utc::now();
    if now < checkpoint.opens_at || now > checkpoint.closes_at {
        return Err(ApiError::new(ErrorCode::CheckpointNotOpen));
    }

    // "Token encontrado no Redis" + "QR não revogado" (spec §9.2 #12/#13):
    // this project has no separate revocation list, so a jti missing from
    // Redis (never issued, or its rotation window already expired) is
    // treated the same as an expired QR.
    let stored_claims = qr::qr_jti_claims(&state, claims.checkpoint_id, &claims.jti).await?;
    if stored_claims.is_none() {
        return Err(ApiError::new(ErrorCode::QrExpired));
    }

    let participant_id = get_or_create_participant(&state.db, &wallet).await?;

    // No `requires_registration` flag exists on `events` (spec §12.3 never
    // defines one) — pragmatically, "inscrição" only matters here in the
    // negative: an organizer who explicitly removed a participant blocks
    // them, otherwise a wallet is implicitly enrolled on its first
    // check-in (mirrors `authz::ensure_personal_organization`'s rationale
    // for filling an unspecified gap rather than blocking the slice).
    let participation_status: Option<String> = sqlx::query_scalar(
        "select status::text from event_participants where event_id = $1 and participant_id = $2",
    )
    .bind(claims.event_id)
    .bind(participant_id)
    .fetch_optional(&state.db)
    .await?;
    match participation_status.as_deref() {
        Some("removed") => return Err(ApiError::new(ErrorCode::ParticipantNotRegistered)),
        Some(_) => {}
        None => {
            sqlx::query(
                "insert into event_participants (event_id, participant_id) values ($1, $2) \
                 on conflict (event_id, participant_id) do nothing",
            )
            .bind(claims.event_id)
            .bind(participant_id)
            .execute(&state.db)
            .await?;
        }
    }

    let grant_id = Uuid::new_v4();
    let challenge_hash = hex::encode(Sha256::digest(body.qr_token.as_bytes()));
    let qr_jti_hash = hex::encode(Sha256::digest(claims.jti.as_bytes()));
    let idempotency_key = grant_id.to_string();
    let expires_at = now + state.config.qr.checkin_grant();

    // Exactly one attempt row per (checkpoint, participant) — spec §11.2's
    // duplicate-check-in guard mirrored off-chain (see the migration's
    // `checkin_attempts_checkpoint_participant_unique` constraint). A new
    // grant may only claim the row when there is no confirmed check-in and
    // no other still-live attempt in flight; that's exactly validations
    // #15/#16.
    let claimed: Option<Uuid> = sqlx::query_scalar(
        "insert into checkin_attempts ( \
             grant_id, event_id, checkpoint_id, participant_id, \
             qr_jti_hash, challenge_hash, idempotency_key, status, expires_at \
         ) values ($1, $2, $3, $4, $5, $6, $7, 'qr_validated', $8) \
         on conflict (checkpoint_id, participant_id) do update set \
             grant_id = excluded.grant_id, \
             qr_jti_hash = excluded.qr_jti_hash, \
             challenge_hash = excluded.challenge_hash, \
             idempotency_key = excluded.idempotency_key, \
             status = 'qr_validated', \
             transaction_signature = null, \
             failure_code = null, \
             failure_message = null, \
             expires_at = excluded.expires_at \
         where checkin_attempts.status in ('failed', 'expired', 'rejected') \
            or checkin_attempts.expires_at < now() \
         returning grant_id",
    )
    .bind(grant_id)
    .bind(claims.event_id)
    .bind(claims.checkpoint_id)
    .bind(participant_id)
    .bind(&qr_jti_hash)
    .bind(&challenge_hash)
    .bind(&idempotency_key)
    .bind(expires_at)
    .fetch_optional(&state.db)
    .await?;

    if claimed.is_none() {
        let existing_status: Option<String> = sqlx::query_scalar(
            "select status::text from checkin_attempts \
             where checkpoint_id = $1 and participant_id = $2",
        )
        .bind(claims.checkpoint_id)
        .bind(participant_id)
        .fetch_optional(&state.db)
        .await?;
        return match existing_status.as_deref() {
            Some("confirmed") => Err(ApiError::new(ErrorCode::AlreadyCheckedIn)),
            _ => Err(ApiError::new(ErrorCode::CheckinPending)),
        };
    }

    let grant = CheckinGrant {
        grant_id,
        event_id: claims.event_id,
        checkpoint_id: claims.checkpoint_id,
        participant_id,
        wallet: wallet.clone(),
        challenge_hash: challenge_hash.clone(),
        idempotency_key,
        expires_at,
    };
    let mut redis = state.redis.clone();
    grant::store_grant(&mut redis, &grant, state.config.qr.checkin_grant_seconds).await?;

    Ok(Json(GrantResponse {
        grant_id,
        event_id: claims.event_id,
        checkpoint_id: claims.checkpoint_id,
        wallet,
        challenge_hash,
        expires_at,
    }))
}

async fn load_grant_for_wallet(
    state: &AppState,
    grant_id: Uuid,
    wallet: &str,
) -> Result<CheckinGrant, ApiError> {
    let mut redis = state.redis.clone();
    let grant = grant::load_grant(&mut redis, grant_id)
        .await?
        .ok_or_else(|| ApiError::new(ErrorCode::GrantExpired))?;
    if grant.wallet != wallet {
        return Err(ApiError::new(ErrorCode::WalletMismatch));
    }
    if grant.expires_at < Utc::now() {
        return Err(ApiError::new(ErrorCode::GrantExpired));
    }
    Ok(grant)
}

/// spec §9.3: `POST /check-ins/{grantId}/prepare-transaction`. Builds the
/// `check_in` instruction (prepending `join_event` the first time this
/// wallet ever checks in to this event — see
/// `programs/eventquest/src/instructions/join_event.rs`'s own header
/// comment), has the attestor co-sign, and returns it base64-encoded for
/// the participant's wallet to sign.
pub async fn prepare_transaction(
    State(state): State<AppState>,
    AuthenticatedWallet(wallet): AuthenticatedWallet,
    Path(grant_id): Path<Uuid>,
) -> Result<Json<PrepareTransactionResponse>, ApiError> {
    let grant = load_grant_for_wallet(&state, grant_id, &wallet).await?;

    let status: Option<String> =
        sqlx::query_scalar("select status::text from checkin_attempts where grant_id = $1")
            .bind(grant_id)
            .fetch_optional(&state.db)
            .await?;
    match status.as_deref() {
        Some("qr_validated") | Some("transaction_prepared") => {}
        Some("confirmed") => return Err(ApiError::new(ErrorCode::AlreadyCheckedIn)),
        Some("failed") => return Err(ApiError::new(ErrorCode::TransactionFailed)),
        Some("rejected") => return Err(ApiError::new(ErrorCode::TransactionRejected)),
        _ => return Err(ApiError::new(ErrorCode::GrantExpired)),
    }

    let event = load_event(&state.db, grant.event_id).await?;
    let checkpoint = load_checkpoint(&state.db, grant.checkpoint_id).await?;

    let event_pubkey = event
        .onchain_event_address
        .as_deref()
        .ok_or_else(|| {
            ApiError::with_message(
                ErrorCode::ChainUnavailable,
                "this event has not been provisioned on-chain yet",
            )
        })
        .and_then(parse_pubkey)?;
    let checkpoint_pubkey = checkpoint
        .onchain_checkpoint_address
        .as_deref()
        .ok_or_else(|| {
            ApiError::with_message(
                ErrorCode::ChainUnavailable,
                "this checkpoint has not been provisioned on-chain yet",
            )
        })
        .and_then(parse_pubkey)?;
    let participant_pubkey = parse_pubkey(&wallet)?;

    if checkpoint.attestor_pubkey != state.attestor_pubkey {
        // Every checkpoint is created with the single global attestor key
        // (see checkpoints::handlers::create_checkpoint) — if this ever
        // diverges, co-signing would build a transaction the on-chain
        // program is guaranteed to reject, so fail fast with a clear
        // internal error instead of wasting a Devnet transaction.
        tracing::error!(
            checkpoint_attestor = %checkpoint.attestor_pubkey,
            configured_attestor = %state.attestor_pubkey,
            "checkpoint attestor does not match the configured attestor keypair"
        );
        return Err(ApiError::new(ErrorCode::InternalError));
    }

    let (participant_event_pubkey, _) = pda::participant_event(&event_pubkey, &participant_pubkey);
    let (attendance_pubkey, _) =
        pda::attendance(&event_pubkey, &checkpoint_pubkey, &participant_pubkey);

    if state
        .blockchain
        .fetch_account::<eventquest_chain::accounts::AttendanceAccount>(attendance_pubkey)
        .await?
        .is_some()
    {
        sqlx::query(
            "update checkin_attempts set status = 'confirmed' where grant_id = $1 and status <> 'confirmed'",
        )
        .bind(grant_id)
        .execute(&state.db)
        .await?;
        return Err(ApiError::new(ErrorCode::AlreadyCheckedIn));
    }

    let participant_joined = state
        .blockchain
        .fetch_account::<eventquest_chain::accounts::ParticipantEventAccount>(
            participant_event_pubkey,
        )
        .await?
        .is_some();

    let mut instructions = Vec::with_capacity(2);
    if !participant_joined {
        let join_accounts = accounts::JoinEvent {
            participant: participant_pubkey,
            event: event_pubkey,
            participant_event: participant_event_pubkey,
            system_program: anchor_lang::system_program::ID,
        };
        instructions.push(anchor_lang::solana_program::instruction::Instruction {
            program_id: state.blockchain.program_id,
            accounts: join_accounts.to_account_metas(None),
            data: args::JoinEvent {}.data(),
        });
    }

    let challenge_hash_bytes: [u8; 32] = hex::decode(&grant.challenge_hash)
        .ok()
        .and_then(|bytes| bytes.try_into().ok())
        .ok_or_else(|| ApiError::new(ErrorCode::InternalError))?;

    let check_in_accounts = accounts::CheckIn {
        participant: participant_pubkey,
        attestor: state.blockchain.attestor_pubkey,
        event: event_pubkey,
        checkpoint: checkpoint_pubkey,
        participant_event: participant_event_pubkey,
        attendance: attendance_pubkey,
        system_program: anchor_lang::system_program::ID,
    };
    instructions.push(anchor_lang::solana_program::instruction::Instruction {
        program_id: state.blockchain.program_id,
        accounts: check_in_accounts.to_account_metas(None),
        data: args::CheckIn {
            challenge_hash: challenge_hash_bytes,
        }
        .data(),
    });

    let transaction = state
        .blockchain
        .build_and_co_sign(&instructions, participant_pubkey)
        .await?;

    sqlx::query("update checkin_attempts set status = 'transaction_prepared' where grant_id = $1")
        .bind(grant_id)
        .execute(&state.db)
        .await?;

    Ok(Json(PrepareTransactionResponse {
        transaction,
        network: "devnet",
        expires_at: grant.expires_at,
        grant_id,
    }))
}

/// spec §9.5: `POST /check-ins/{grantId}/submitted`. Records the
/// participant-submitted signature and does a one-shot check for an
/// immediately-known failure; the transition to `CONFIRMED` is left to
/// `apps/indexer` (spec §16 — "o indexador on-chain é a fonte final da
/// confirmação").
pub async fn submitted(
    State(state): State<AppState>,
    AuthenticatedWallet(wallet): AuthenticatedWallet,
    Path(grant_id): Path<Uuid>,
    Json(body): Json<SubmittedRequest>,
) -> Result<Json<CheckinStatusResponse>, ApiError> {
    let grant = load_grant_for_wallet(&state, grant_id, &wallet).await?;

    let status: Option<String> =
        sqlx::query_scalar("select status::text from checkin_attempts where grant_id = $1")
            .bind(grant_id)
            .fetch_optional(&state.db)
            .await?;
    match status.as_deref() {
        Some("transaction_prepared") => {}
        Some("transaction_submitted") | Some("confirmed") => {
            return get_status(State(state), AuthenticatedWallet(wallet), Path(grant_id)).await;
        }
        _ => {
            return Err(ApiError::with_message(
                ErrorCode::ValidationError,
                "call prepare-transaction before submitted",
            ))
        }
    }

    let signature_bytes = bs58::decode(&body.signature)
        .into_vec()
        .map_err(|_| ApiError::with_message(ErrorCode::ValidationError, "invalid signature"))?;
    if signature_bytes.len() != 64 {
        return Err(ApiError::with_message(
            ErrorCode::ValidationError,
            "invalid signature",
        ));
    }

    if let Some(err) = state.blockchain.signature_error(&body.signature).await? {
        let failure_message = err.to_string();
        sqlx::query(
            "update checkin_attempts set status = 'failed', transaction_signature = $1, \
             failure_code = 'TRANSACTION_FAILED', failure_message = $2 where grant_id = $3",
        )
        .bind(&body.signature)
        .bind(&failure_message)
        .bind(grant_id)
        .execute(&state.db)
        .await?;
        return Err(ApiError::with_message(
            ErrorCode::TransactionFailed,
            failure_message,
        ));
    }

    sqlx::query(
        "update checkin_attempts set status = 'transaction_submitted', transaction_signature = $1 \
         where grant_id = $2",
    )
    .bind(&body.signature)
    .bind(grant_id)
    .execute(&state.db)
    .await?;

    Ok(Json(CheckinStatusResponse {
        grant_id,
        status: "transaction_submitted".to_string(),
        transaction_signature: Some(body.signature),
        failure_code: None,
        failure_message: None,
        expires_at: grant.expires_at,
    }))
}

#[derive(sqlx::FromRow)]
struct CheckinStatusRow {
    status: String,
    transaction_signature: Option<String>,
    failure_code: Option<String>,
    failure_message: Option<String>,
    expires_at: chrono::DateTime<Utc>,
    wallet_address: String,
}

/// `GET /check-ins/{grantId}` — lets the frontend poll for the eventual
/// `CONFIRMED` transition that `apps/indexer` applies (spec §9.5/§16).
pub async fn get_status(
    State(state): State<AppState>,
    AuthenticatedWallet(wallet): AuthenticatedWallet,
    Path(grant_id): Path<Uuid>,
) -> Result<Json<CheckinStatusResponse>, ApiError> {
    let row = sqlx::query_as::<_, CheckinStatusRow>(
        "select ca.status::text, ca.transaction_signature, ca.failure_code, \
             ca.failure_message, ca.expires_at, p.wallet_address \
             from checkin_attempts ca join participants p on p.id = ca.participant_id \
             where ca.grant_id = $1",
    )
    .bind(grant_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::new(ErrorCode::ResourceNotFound))?;

    if row.wallet_address != wallet {
        return Err(ApiError::new(ErrorCode::ResourceNotFound));
    }

    Ok(Json(CheckinStatusResponse {
        grant_id,
        status: row.status,
        transaction_signature: row.transaction_signature,
        failure_code: row.failure_code,
        failure_message: row.failure_message,
        expires_at: row.expires_at,
    }))
}
