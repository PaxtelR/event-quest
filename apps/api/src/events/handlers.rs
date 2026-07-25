use anchor_lang::solana_program::instruction::Instruction;
use anchor_lang::{AnchorSerialize, InstructionData, ToAccountMetas};

/// The client-generated `EventStatus` (unlike the on-chain program's own
/// hand-written one) doesn't derive `PartialEq` — compare via its stable
/// Borsh encoding instead.
fn same_event_status(a: &ChainEventStatus, b: &ChainEventStatus) -> bool {
    let mut a_bytes = Vec::new();
    let mut b_bytes = Vec::new();
    a.serialize(&mut a_bytes).is_ok() && b.serialize(&mut b_bytes).is_ok() && a_bytes == b_bytes
}
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::{DateTime, Utc};
use eventquest_chain::client::{accounts, args};
use eventquest_chain::{pda, types::EventStatus as ChainEventStatus, ID as PROGRAM_ID};
use eventquest_domain::ErrorCode;
use uuid::Uuid;
use validator::Validate;

use crate::auth::{AuthenticatedWallet, OptionalAuthenticatedWallet};
use crate::authz;
use crate::blockchain::parse_pubkey;
use crate::error::ApiError;
use crate::onchain::{self, PrepareResponse, SubmittedRequest};
use crate::state::AppState;

use super::models::{
    CreateEventRequest, EventDto, ListEventsQuery, UpdateEventRequest, EVENT_COLUMNS,
};

fn validation_error(err: validator::ValidationErrors) -> ApiError {
    ApiError::with_message(ErrorCode::ValidationError, err.to_string())
}

pub async fn create_event(
    State(state): State<AppState>,
    AuthenticatedWallet(wallet): AuthenticatedWallet,
    Json(body): Json<CreateEventRequest>,
) -> Result<(StatusCode, Json<EventDto>), ApiError> {
    body.validate().map_err(validation_error)?;

    if body.starts_at >= body.ends_at {
        return Err(ApiError::with_message(
            ErrorCode::ValidationError,
            "startsAt must be before endsAt",
        ));
    }

    let organization_id = match body.organization_id {
        Some(organization_id) => {
            authz::require_org_admin(&state.db, organization_id, &wallet).await?;
            organization_id
        }
        None => authz::ensure_personal_organization(&state.db, &wallet).await?,
    };

    let visibility = body.visibility.map(|v| v.as_db_str()).unwrap_or("private");
    let timezone = body.timezone.as_deref().unwrap_or("UTC");

    let query = format!(
        "insert into events (organization_id, name, slug, description, banner_url, \
         location_name, location_address, timezone, starts_at, ends_at, visibility, created_by_wallet) \
         values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11::event_visibility, $12) \
         returning {EVENT_COLUMNS}"
    );

    let event = sqlx::query_as::<_, EventDto>(&query)
        .bind(organization_id)
        .bind(&body.name)
        .bind(&body.slug)
        .bind(&body.description)
        .bind(&body.banner_url)
        .bind(&body.location_name)
        .bind(&body.location_address)
        .bind(timezone)
        .bind(body.starts_at)
        .bind(body.ends_at)
        .bind(visibility)
        .bind(&wallet)
        .fetch_one(&state.db)
        .await
        .map_err(map_insert_error)?;

    Ok((StatusCode::CREATED, Json(event)))
}

pub async fn list_events(
    State(state): State<AppState>,
    OptionalAuthenticatedWallet(wallet): OptionalAuthenticatedWallet,
    Query(query): Query<ListEventsQuery>,
) -> Result<Json<Vec<EventDto>>, ApiError> {
    let is_member = match (wallet.as_deref(), query.organization_id) {
        (Some(wallet), Some(organization_id)) => {
            authz::require_org_member(&state.db, organization_id, wallet)
                .await
                .is_ok()
        }
        _ => false,
    };

    let events = if is_member {
        let sql = format!(
            "select {EVENT_COLUMNS} from events where organization_id = $1 order by starts_at desc"
        );
        sqlx::query_as::<_, EventDto>(&sql)
            .bind(query.organization_id.expect("checked above"))
            .fetch_all(&state.db)
            .await?
    } else {
        // Public listing: only events visible to anyone, optionally
        // narrowed to one organization.
        let sql = format!(
            "select {EVENT_COLUMNS} from events \
             where visibility = 'public' and status in ('active', 'paused', 'finished') \
             and ($1::uuid is null or organization_id = $1) \
             order by starts_at desc"
        );
        sqlx::query_as::<_, EventDto>(&sql)
            .bind(query.organization_id)
            .fetch_all(&state.db)
            .await?
    };

    Ok(Json(events))
}

pub async fn get_event(
    State(state): State<AppState>,
    OptionalAuthenticatedWallet(wallet): OptionalAuthenticatedWallet,
    Path(event_id): Path<Uuid>,
) -> Result<Json<EventDto>, ApiError> {
    let sql = format!("select {EVENT_COLUMNS} from events where id = $1");
    let event = sqlx::query_as::<_, EventDto>(&sql)
        .bind(event_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::new(ErrorCode::ResourceNotFound))?;

    let publicly_visible = event.visibility == "public"
        && matches!(event.status.as_str(), "active" | "paused" | "finished");

    if !publicly_visible {
        let wallet = wallet.ok_or_else(|| ApiError::new(ErrorCode::ResourceNotFound))?;
        authz::require_org_member(&state.db, event.organization_id, &wallet)
            .await
            .map_err(|_| ApiError::new(ErrorCode::ResourceNotFound))?;
    }

    Ok(Json(event))
}

pub async fn update_event(
    State(state): State<AppState>,
    AuthenticatedWallet(wallet): AuthenticatedWallet,
    Path(event_id): Path<Uuid>,
    Json(body): Json<UpdateEventRequest>,
) -> Result<Json<EventDto>, ApiError> {
    body.validate().map_err(validation_error)?;

    let organization_id = authz::event_organization_id(&state.db, event_id).await?;
    authz::require_org_admin(&state.db, organization_id, &wallet).await?;

    if let (Some(starts_at), Some(ends_at)) = (body.starts_at, body.ends_at) {
        if starts_at >= ends_at {
            return Err(ApiError::with_message(
                ErrorCode::ValidationError,
                "startsAt must be before endsAt",
            ));
        }
    }

    let visibility = body.visibility.map(|v| v.as_db_str());

    let sql = format!(
        "update events set \
         name = coalesce($1, name), \
         description = coalesce($2, description), \
         banner_url = coalesce($3, banner_url), \
         location_name = coalesce($4, location_name), \
         location_address = coalesce($5, location_address), \
         timezone = coalesce($6, timezone), \
         starts_at = coalesce($7, starts_at), \
         ends_at = coalesce($8, ends_at), \
         visibility = coalesce($9::event_visibility, visibility) \
         where id = $10 \
         returning {EVENT_COLUMNS}"
    );

    let event = sqlx::query_as::<_, EventDto>(&sql)
        .bind(&body.name)
        .bind(&body.description)
        .bind(&body.banner_url)
        .bind(&body.location_name)
        .bind(&body.location_address)
        .bind(&body.timezone)
        .bind(body.starts_at)
        .bind(body.ends_at)
        .bind(visibility)
        .bind(event_id)
        .fetch_one(&state.db)
        .await?;

    Ok(Json(event))
}

// ---------------------------------------------------------------------
// Status transitions (publish/pause/finish) — spec §11.3's
// `initialize_event`/`update_event_status`. `check_in`/`join_event`
// validate `event.status == Active` against the ON-CHAIN account, never
// this off-chain column, so every transition that can reach or leave
// `active` has to be backed by a real organizer-signed transaction: the
// off-chain status only ever changes *after* confirming the matching
// on-chain state, via the shared `onchain` module's prepare/submitted
// pattern (see apps/api/src/onchain.rs and apps/api/src/checkins/mod.rs's
// header comment for the gap this closes).
// ---------------------------------------------------------------------

#[derive(sqlx::FromRow)]
struct EventChainRow {
    organization_id: Uuid,
    created_by_wallet: String,
    starts_at: DateTime<Utc>,
    ends_at: DateTime<Utc>,
    status: String,
    onchain_event_address: Option<String>,
}

async fn load_event_chain_row(state: &AppState, event_id: Uuid) -> Result<EventChainRow, ApiError> {
    sqlx::query_as::<_, EventChainRow>(
        "select organization_id, created_by_wallet, starts_at, ends_at, \
         status::text as status, onchain_event_address from events where id = $1",
    )
    .bind(event_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::new(ErrorCode::ResourceNotFound))
}

/// Direct off-chain-only transition — only ever reached for a `draft`
/// event with nothing on-chain yet to protect (see the invariant argued
/// in `prepare_status_change` below: `active`/`paused` can't be reached
/// without on-chain provisioning already having happened).
async fn apply_status_transition(
    state: &AppState,
    event_id: Uuid,
    target_status: &str,
    allowed_from: &[&str],
) -> Result<EventDto, ApiError> {
    let allowed_list = format!("{{{}}}", allowed_from.join(","));
    let sql = format!(
        "update events set status = $1::event_status \
         where id = $2 and status::text = any($3::text[]) \
         returning {EVENT_COLUMNS}"
    );
    sqlx::query_as::<_, EventDto>(&sql)
        .bind(target_status)
        .bind(event_id)
        .bind(allowed_list)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::new(ErrorCode::ValidationError))
}

async fn prepare_status_change(
    state: &AppState,
    wallet: &str,
    event_id: Uuid,
    chain_status: ChainEventStatus,
    target_status: &str,
    allowed_from: &[&str],
    bundle_init_if_missing: bool,
) -> Result<PrepareResponse<EventDto>, ApiError> {
    let row = load_event_chain_row(state, event_id).await?;
    authz::require_org_admin(&state.db, row.organization_id, wallet).await?;
    if !allowed_from.contains(&row.status.as_str()) {
        return Err(ApiError::new(ErrorCode::ValidationError));
    }

    let organizer = parse_pubkey(&row.created_by_wallet)?;
    let external_id_hash = onchain::external_id_hash(event_id);
    let (event_pda, _) = pda::event(&organizer, &external_id_hash);

    if row.onchain_event_address.is_none() {
        if !bundle_init_if_missing {
            let event =
                apply_status_transition(state, event_id, target_status, allowed_from).await?;
            return Ok(PrepareResponse::applied(event));
        }

        let instructions = [
            Instruction {
                program_id: PROGRAM_ID,
                accounts: accounts::InitializeEvent {
                    authority: organizer,
                    event: event_pda,
                    system_program: anchor_lang::system_program::ID,
                }
                .to_account_metas(None),
                data: args::InitializeEvent {
                    external_id_hash,
                    starts_at: row.starts_at.timestamp(),
                    ends_at: row.ends_at.timestamp(),
                }
                .data(),
            },
            Instruction {
                program_id: PROGRAM_ID,
                accounts: accounts::UpdateEventStatus {
                    authority: organizer,
                    event: event_pda,
                }
                .to_account_metas(None),
                data: args::UpdateEventStatus {
                    new_status: chain_status,
                }
                .data(),
            },
        ];
        let transaction = state
            .blockchain
            .build_unsigned(&instructions, organizer)
            .await?;
        return Ok(PrepareResponse::requires_signature(transaction));
    }

    let instruction = Instruction {
        program_id: PROGRAM_ID,
        accounts: accounts::UpdateEventStatus {
            authority: organizer,
            event: event_pda,
        }
        .to_account_metas(None),
        data: args::UpdateEventStatus {
            new_status: chain_status,
        }
        .data(),
    };
    let transaction = state
        .blockchain
        .build_unsigned(&[instruction], organizer)
        .await?;
    Ok(PrepareResponse::requires_signature(transaction))
}

async fn status_change_submitted(
    state: &AppState,
    wallet: &str,
    event_id: Uuid,
    signature: &str,
    chain_status: ChainEventStatus,
    target_status: &str,
    allowed_from: &[&str],
) -> Result<EventDto, ApiError> {
    let row = load_event_chain_row(state, event_id).await?;
    authz::require_org_admin(&state.db, row.organization_id, wallet).await?;

    let organizer = parse_pubkey(&row.created_by_wallet)?;
    let external_id_hash = onchain::external_id_hash(event_id);
    let (event_pda, _) = pda::event(&organizer, &external_id_hash);

    let account: eventquest_chain::accounts::EventAccount =
        onchain::confirm_and_fetch(state, signature, event_pda).await?;
    if account.authority != organizer
        || account.external_id_hash != external_id_hash
        || !same_event_status(&account.status, &chain_status)
    {
        return Err(ApiError::new(ErrorCode::TransactionFailed));
    }

    let allowed_list = format!("{{{}}}", allowed_from.join(","));
    let sql = format!(
        "update events set status = $1::event_status, \
         onchain_event_address = coalesce(onchain_event_address, $2), \
         onchain_create_signature = coalesce(onchain_create_signature, $3) \
         where id = $4 and status::text = any($5::text[]) \
         returning {EVENT_COLUMNS}"
    );
    sqlx::query_as::<_, EventDto>(&sql)
        .bind(target_status)
        .bind(event_pda.to_string())
        .bind(signature)
        .bind(event_id)
        .bind(allowed_list)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::new(ErrorCode::ValidationError))
}

const PUBLISH_ALLOWED_FROM: &[&str] = &["draft", "paused"];
const PAUSE_ALLOWED_FROM: &[&str] = &["active"];
const FINISH_ALLOWED_FROM: &[&str] = &["active", "paused"];

pub async fn prepare_publish(
    State(state): State<AppState>,
    AuthenticatedWallet(wallet): AuthenticatedWallet,
    Path(event_id): Path<Uuid>,
) -> Result<Json<PrepareResponse<EventDto>>, ApiError> {
    let response = prepare_status_change(
        &state,
        &wallet,
        event_id,
        ChainEventStatus::Active,
        "active",
        PUBLISH_ALLOWED_FROM,
        true,
    )
    .await?;
    Ok(Json(response))
}

pub async fn publish_submitted(
    State(state): State<AppState>,
    AuthenticatedWallet(wallet): AuthenticatedWallet,
    Path(event_id): Path<Uuid>,
    Json(body): Json<SubmittedRequest>,
) -> Result<Json<EventDto>, ApiError> {
    let event = status_change_submitted(
        &state,
        &wallet,
        event_id,
        &body.signature,
        ChainEventStatus::Active,
        "active",
        PUBLISH_ALLOWED_FROM,
    )
    .await?;
    Ok(Json(event))
}

pub async fn prepare_pause(
    State(state): State<AppState>,
    AuthenticatedWallet(wallet): AuthenticatedWallet,
    Path(event_id): Path<Uuid>,
) -> Result<Json<PrepareResponse<EventDto>>, ApiError> {
    let response = prepare_status_change(
        &state,
        &wallet,
        event_id,
        ChainEventStatus::Paused,
        "paused",
        PAUSE_ALLOWED_FROM,
        false,
    )
    .await?;
    Ok(Json(response))
}

pub async fn pause_submitted(
    State(state): State<AppState>,
    AuthenticatedWallet(wallet): AuthenticatedWallet,
    Path(event_id): Path<Uuid>,
    Json(body): Json<SubmittedRequest>,
) -> Result<Json<EventDto>, ApiError> {
    let event = status_change_submitted(
        &state,
        &wallet,
        event_id,
        &body.signature,
        ChainEventStatus::Paused,
        "paused",
        PAUSE_ALLOWED_FROM,
    )
    .await?;
    Ok(Json(event))
}

pub async fn prepare_finish(
    State(state): State<AppState>,
    AuthenticatedWallet(wallet): AuthenticatedWallet,
    Path(event_id): Path<Uuid>,
) -> Result<Json<PrepareResponse<EventDto>>, ApiError> {
    let response = prepare_status_change(
        &state,
        &wallet,
        event_id,
        ChainEventStatus::Finished,
        "finished",
        FINISH_ALLOWED_FROM,
        false,
    )
    .await?;
    Ok(Json(response))
}

pub async fn finish_submitted(
    State(state): State<AppState>,
    AuthenticatedWallet(wallet): AuthenticatedWallet,
    Path(event_id): Path<Uuid>,
    Json(body): Json<SubmittedRequest>,
) -> Result<Json<EventDto>, ApiError> {
    let event = status_change_submitted(
        &state,
        &wallet,
        event_id,
        &body.signature,
        ChainEventStatus::Finished,
        "finished",
        FINISH_ALLOWED_FROM,
    )
    .await?;
    Ok(Json(event))
}

fn map_insert_error(err: sqlx::Error) -> ApiError {
    if let sqlx::Error::Database(db_err) = &err {
        if db_err.constraint().is_some() {
            return ApiError::with_message(
                ErrorCode::ValidationError,
                "an event with this slug already exists in this organization",
            );
        }
    }
    ApiError::from(err)
}
