use anchor_lang::solana_program::instruction::Instruction;
use anchor_lang::{InstructionData, ToAccountMetas};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::{DateTime, Utc};
use eventquest_chain::client::{accounts, args};
use eventquest_chain::constants::MAX_POINTS_PER_CHECKPOINT;
use eventquest_chain::types::UpdateCheckpointParams;
use eventquest_chain::{pda, ID as PROGRAM_ID};
use eventquest_domain::ErrorCode;
use uuid::Uuid;
use validator::Validate;

use crate::auth::AuthenticatedWallet;
use crate::authz;
use crate::blockchain::parse_pubkey;
use crate::error::ApiError;
use crate::onchain::{self, PrepareResponse, SubmittedRequest};
use crate::state::AppState;

use super::models::{
    CheckpointDto, CreateCheckpointRequest, UpdateCheckpointRequest, CHECKPOINT_COLUMNS,
};

fn validation_error(err: validator::ValidationErrors) -> ApiError {
    ApiError::with_message(ErrorCode::ValidationError, err.to_string())
}

const DEFAULT_ROTATION_SECONDS: i32 = 15;

pub async fn create_checkpoint(
    State(state): State<AppState>,
    AuthenticatedWallet(wallet): AuthenticatedWallet,
    Path(event_id): Path<Uuid>,
    Json(body): Json<CreateCheckpointRequest>,
) -> Result<(StatusCode, Json<CheckpointDto>), ApiError> {
    body.validate().map_err(validation_error)?;

    if body.points as u32 > MAX_POINTS_PER_CHECKPOINT {
        return Err(ApiError::with_message(
            ErrorCode::ValidationError,
            format!("points must not exceed {MAX_POINTS_PER_CHECKPOINT}"),
        ));
    }
    if body.opens_at >= body.closes_at {
        return Err(ApiError::with_message(
            ErrorCode::ValidationError,
            "opensAt must be before closesAt",
        ));
    }

    let organization_id = authz::event_organization_id(&state.db, event_id).await?;
    authz::require_org_admin(&state.db, organization_id, &wallet).await?;

    let (event_starts_at, event_ends_at): (
        chrono::DateTime<chrono::Utc>,
        chrono::DateTime<chrono::Utc>,
    ) = sqlx::query_as("select starts_at, ends_at from events where id = $1")
        .bind(event_id)
        .fetch_one(&state.db)
        .await?;
    if body.opens_at < event_starts_at || body.closes_at > event_ends_at {
        return Err(ApiError::with_message(
            ErrorCode::ValidationError,
            "checkpoint window must fall within the event's period",
        ));
    }

    let configured_default_rotation: i32 = state
        .config
        .qr
        .rotation_seconds
        .try_into()
        .unwrap_or(DEFAULT_ROTATION_SECONDS);
    let rotation_seconds = body.rotation_seconds.unwrap_or(configured_default_rotation);

    let sql = format!(
        "insert into checkpoints (event_id, name, description, points, rotation_seconds, \
         opens_at, closes_at, attestor_pubkey) \
         values ($1, $2, $3, $4, $5, $6, $7, $8) \
         returning {CHECKPOINT_COLUMNS}"
    );

    let checkpoint = sqlx::query_as::<_, CheckpointDto>(&sql)
        .bind(event_id)
        .bind(&body.name)
        .bind(&body.description)
        .bind(body.points)
        .bind(rotation_seconds)
        .bind(body.opens_at)
        .bind(body.closes_at)
        .bind(&state.attestor_pubkey)
        .fetch_one(&state.db)
        .await?;

    Ok((StatusCode::CREATED, Json(checkpoint)))
}

pub async fn list_checkpoints(
    State(state): State<AppState>,
    Path(event_id): Path<Uuid>,
) -> Result<Json<Vec<CheckpointDto>>, ApiError> {
    let sql = format!(
        "select {CHECKPOINT_COLUMNS} from checkpoints where event_id = $1 order by opens_at asc"
    );
    let checkpoints = sqlx::query_as::<_, CheckpointDto>(&sql)
        .bind(event_id)
        .fetch_all(&state.db)
        .await?;
    Ok(Json(checkpoints))
}

pub async fn get_checkpoint(
    State(state): State<AppState>,
    Path(checkpoint_id): Path<Uuid>,
) -> Result<Json<CheckpointDto>, ApiError> {
    let sql = format!("select {CHECKPOINT_COLUMNS} from checkpoints where id = $1");
    let checkpoint = sqlx::query_as::<_, CheckpointDto>(&sql)
        .bind(checkpoint_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::new(ErrorCode::ResourceNotFound))?;
    Ok(Json(checkpoint))
}

pub async fn update_checkpoint(
    State(state): State<AppState>,
    AuthenticatedWallet(wallet): AuthenticatedWallet,
    Path(checkpoint_id): Path<Uuid>,
    Json(body): Json<UpdateCheckpointRequest>,
) -> Result<Json<CheckpointDto>, ApiError> {
    body.validate().map_err(validation_error)?;

    let organization_id = authz::checkpoint_organization_id(&state.db, checkpoint_id).await?;
    authz::require_org_admin(&state.db, organization_id, &wallet).await?;

    if let Some(points) = body.points {
        if points as u32 > MAX_POINTS_PER_CHECKPOINT {
            return Err(ApiError::with_message(
                ErrorCode::ValidationError,
                format!("points must not exceed {MAX_POINTS_PER_CHECKPOINT}"),
            ));
        }

        // Mirrors the on-chain rule (programs/eventquest/src/instructions/
        // update_checkpoint.rs): once a checkpoint has confirmed
        // attendances, points can never change retroactively.
        let has_attendances: bool =
            sqlx::query_scalar("select exists(select 1 from attendances where checkpoint_id = $1)")
                .bind(checkpoint_id)
                .fetch_one(&state.db)
                .await?;
        if has_attendances {
            return Err(ApiError::with_message(
                ErrorCode::ValidationError,
                "points cannot be changed after check-ins have been confirmed",
            ));
        }
    }

    // Note: this only updates the off-chain row. If the checkpoint is
    // already provisioned on-chain, `points`/`opens_at`/`closes_at` here
    // will drift from the on-chain `CheckpointAccount` until an on-chain
    // `update_checkpoint` call is made to match — that call isn't wired
    // to this endpoint yet (a real, separate gap from the provisioning
    // one `prepare_activate`/`prepare_pause` below close; see this
    // module's `mod.rs` header comment).
    let sql = format!(
        "update checkpoints set \
         name = coalesce($1, name), \
         description = coalesce($2, description), \
         points = coalesce($3, points), \
         rotation_seconds = coalesce($4, rotation_seconds), \
         opens_at = coalesce($5, opens_at), \
         closes_at = coalesce($6, closes_at) \
         where id = $7 \
         returning {CHECKPOINT_COLUMNS}"
    );

    let checkpoint = sqlx::query_as::<_, CheckpointDto>(&sql)
        .bind(&body.name)
        .bind(&body.description)
        .bind(body.points)
        .bind(body.rotation_seconds)
        .bind(body.opens_at)
        .bind(body.closes_at)
        .bind(checkpoint_id)
        .fetch_one(&state.db)
        .await?;

    Ok(Json(checkpoint))
}

// ---------------------------------------------------------------------
// Active/paused transitions — spec §11.3's `create_checkpoint`/
// `update_checkpoint`. `check_in` validates `checkpoint.active` against
// the ON-CHAIN account, never this off-chain column, so — exactly like
// events' publish/pause/finish — the off-chain status only changes after
// confirming the matching on-chain state. See apps/api/src/onchain.rs and
// apps/api/src/checkins/mod.rs's header comment for the gap this closes.
// ---------------------------------------------------------------------

#[derive(sqlx::FromRow)]
struct CheckpointChainRow {
    event_id: Uuid,
    points: i32,
    opens_at: DateTime<Utc>,
    closes_at: DateTime<Utc>,
    attestor_pubkey: String,
    onchain_checkpoint_address: Option<String>,
}

async fn load_checkpoint_chain_row(
    state: &AppState,
    checkpoint_id: Uuid,
) -> Result<CheckpointChainRow, ApiError> {
    sqlx::query_as::<_, CheckpointChainRow>(
        "select event_id, points, opens_at, closes_at, attestor_pubkey, \
         onchain_checkpoint_address from checkpoints where id = $1",
    )
    .bind(checkpoint_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::new(ErrorCode::ResourceNotFound))
}

#[derive(sqlx::FromRow)]
struct EventChainRefRow {
    created_by_wallet: String,
    onchain_event_address: Option<String>,
}

async fn load_event_chain_ref(
    state: &AppState,
    event_id: Uuid,
) -> Result<EventChainRefRow, ApiError> {
    sqlx::query_as::<_, EventChainRefRow>(
        "select created_by_wallet, onchain_event_address from events where id = $1",
    )
    .bind(event_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::new(ErrorCode::ResourceNotFound))
}

async fn apply_checkpoint_status(
    state: &AppState,
    checkpoint_id: Uuid,
    target_status: &str,
) -> Result<CheckpointDto, ApiError> {
    let sql = format!(
        "update checkpoints set status = $1::checkpoint_status where id = $2 \
         returning {CHECKPOINT_COLUMNS}"
    );
    sqlx::query_as::<_, CheckpointDto>(&sql)
        .bind(target_status)
        .bind(checkpoint_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::new(ErrorCode::ResourceNotFound))
}

async fn prepare_active_change(
    state: &AppState,
    wallet: &str,
    checkpoint_id: Uuid,
    active: bool,
    target_status: &str,
) -> Result<PrepareResponse<CheckpointDto>, ApiError> {
    // Operators (spec §5.3) may activate/pause a checkpoint without full
    // event-admin rights; admins/owners can too.
    let organization_id = authz::checkpoint_organization_id(&state.db, checkpoint_id).await?;
    authz::require_org_member(&state.db, organization_id, wallet).await?;

    let row = load_checkpoint_chain_row(state, checkpoint_id).await?;
    let event_ref = load_event_chain_ref(state, row.event_id).await?;
    let organizer = parse_pubkey(&event_ref.created_by_wallet)?;
    let external_id_hash = onchain::external_id_hash(checkpoint_id);

    if let Some(existing) = &row.onchain_checkpoint_address {
        let event_pda = parse_pubkey(
            event_ref
                .onchain_event_address
                .as_deref()
                .ok_or_else(|| ApiError::new(ErrorCode::InternalError))?,
        )?;
        let checkpoint_pda = parse_pubkey(existing)?;

        let instruction = Instruction {
            program_id: PROGRAM_ID,
            accounts: accounts::UpdateCheckpoint {
                authority: organizer,
                event: event_pda,
                checkpoint: checkpoint_pda,
            }
            .to_account_metas(None),
            data: args::UpdateCheckpoint {
                params: UpdateCheckpointParams {
                    attestor: None,
                    opens_at: None,
                    closes_at: None,
                    points: None,
                    active: Some(active),
                },
            }
            .data(),
        };
        let transaction = state
            .blockchain
            .build_unsigned(&[instruction], organizer)
            .await?;
        return Ok(PrepareResponse::requires_signature(transaction));
    }

    if !active {
        // Nothing on-chain yet to pause — pure off-chain transition.
        let checkpoint = apply_checkpoint_status(state, checkpoint_id, target_status).await?;
        return Ok(PrepareResponse::applied(checkpoint));
    }

    let event_pda_str = event_ref.onchain_event_address.ok_or_else(|| {
        ApiError::with_message(
            ErrorCode::ChainUnavailable,
            "the parent event has not been provisioned on-chain yet — publish it first",
        )
    })?;
    let event_pda = parse_pubkey(&event_pda_str)?;
    let (checkpoint_pda, _) = pda::checkpoint(&event_pda, &external_id_hash);
    let attestor = parse_pubkey(&row.attestor_pubkey)?;

    let instruction = Instruction {
        program_id: PROGRAM_ID,
        accounts: accounts::CreateCheckpoint {
            authority: organizer,
            event: event_pda,
            checkpoint: checkpoint_pda,
            system_program: anchor_lang::system_program::ID,
        }
        .to_account_metas(None),
        data: args::CreateCheckpoint {
            external_id_hash,
            attestor,
            opens_at: row.opens_at.timestamp(),
            closes_at: row.closes_at.timestamp(),
            points: row.points as u32,
        }
        .data(),
    };
    let transaction = state
        .blockchain
        .build_unsigned(&[instruction], organizer)
        .await?;
    Ok(PrepareResponse::requires_signature(transaction))
}

async fn active_change_submitted(
    state: &AppState,
    wallet: &str,
    checkpoint_id: Uuid,
    signature: &str,
    active: bool,
    target_status: &str,
) -> Result<CheckpointDto, ApiError> {
    let organization_id = authz::checkpoint_organization_id(&state.db, checkpoint_id).await?;
    authz::require_org_member(&state.db, organization_id, wallet).await?;

    let row = load_checkpoint_chain_row(state, checkpoint_id).await?;
    let event_ref = load_event_chain_ref(state, row.event_id).await?;
    let external_id_hash = onchain::external_id_hash(checkpoint_id);
    let attestor = parse_pubkey(&row.attestor_pubkey)?;

    let checkpoint_pda = match &row.onchain_checkpoint_address {
        Some(existing) => parse_pubkey(existing)?,
        None => {
            let event_pda_str = event_ref
                .onchain_event_address
                .as_deref()
                .ok_or_else(|| ApiError::new(ErrorCode::InternalError))?;
            let event_pda = parse_pubkey(event_pda_str)?;
            pda::checkpoint(&event_pda, &external_id_hash).0
        }
    };

    let account: eventquest_chain::accounts::CheckpointAccount =
        onchain::confirm_and_fetch(state, signature, checkpoint_pda).await?;
    if account.attestor != attestor
        || account.external_id_hash != external_id_hash
        || account.active != active
    {
        return Err(ApiError::new(ErrorCode::TransactionFailed));
    }

    let sql = format!(
        "update checkpoints set status = $1::checkpoint_status, \
         onchain_checkpoint_address = coalesce(onchain_checkpoint_address, $2) \
         where id = $3 \
         returning {CHECKPOINT_COLUMNS}"
    );
    sqlx::query_as::<_, CheckpointDto>(&sql)
        .bind(target_status)
        .bind(checkpoint_pda.to_string())
        .bind(checkpoint_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::new(ErrorCode::ResourceNotFound))
}

pub async fn prepare_activate(
    State(state): State<AppState>,
    AuthenticatedWallet(wallet): AuthenticatedWallet,
    Path(checkpoint_id): Path<Uuid>,
) -> Result<Json<PrepareResponse<CheckpointDto>>, ApiError> {
    let response = prepare_active_change(&state, &wallet, checkpoint_id, true, "active").await?;
    Ok(Json(response))
}

pub async fn activate_submitted(
    State(state): State<AppState>,
    AuthenticatedWallet(wallet): AuthenticatedWallet,
    Path(checkpoint_id): Path<Uuid>,
    Json(body): Json<SubmittedRequest>,
) -> Result<Json<CheckpointDto>, ApiError> {
    let checkpoint = active_change_submitted(
        &state,
        &wallet,
        checkpoint_id,
        &body.signature,
        true,
        "active",
    )
    .await?;
    Ok(Json(checkpoint))
}

pub async fn prepare_pause(
    State(state): State<AppState>,
    AuthenticatedWallet(wallet): AuthenticatedWallet,
    Path(checkpoint_id): Path<Uuid>,
) -> Result<Json<PrepareResponse<CheckpointDto>>, ApiError> {
    let response = prepare_active_change(&state, &wallet, checkpoint_id, false, "paused").await?;
    Ok(Json(response))
}

pub async fn pause_submitted(
    State(state): State<AppState>,
    AuthenticatedWallet(wallet): AuthenticatedWallet,
    Path(checkpoint_id): Path<Uuid>,
    Json(body): Json<SubmittedRequest>,
) -> Result<Json<CheckpointDto>, ApiError> {
    let checkpoint = active_change_submitted(
        &state,
        &wallet,
        checkpoint_id,
        &body.signature,
        false,
        "paused",
    )
    .await?;
    Ok(Json(checkpoint))
}
