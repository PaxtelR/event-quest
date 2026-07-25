//! One indexing pass — spec §16: walk signatures for the program from the
//! persisted cursor, decode `AttendanceRecorded` out of each confirmed
//! transaction's logs, and upsert `attendances` (idempotently — a
//! transaction re-seen after a restart or during reconciliation is a
//! no-op the second time, via the same unique constraints
//! `apps/api`'s own check-in flow relies on).

use anyhow::Result;
use chrono::{TimeZone, Utc};
use eventquest_chain::pda;
use redis::AsyncCommands;
use sqlx::PgPool;
use uuid::Uuid;

use crate::events::find_attendance_recorded;
use crate::rpc::RpcClient;

/// How many signatures to request per `getSignaturesForAddress` page.
/// Solana's own cap is 1000; there's no reason to ask for less; paging
/// (via `before`) only kicks in if more than this many new signatures
/// piled up since the last tick (e.g. after downtime).
const PAGE_SIZE: usize = 1000;

pub struct Cursor {
    pub network: String,
    pub program_id: String,
}

async fn load_cursor_slot(db: &PgPool, cursor: &Cursor) -> Result<i64> {
    let slot: Option<i64> = sqlx::query_scalar(
        "select last_processed_slot from chain_sync_cursors where network = $1 and program_id = $2",
    )
    .bind(&cursor.network)
    .bind(&cursor.program_id)
    .fetch_optional(db)
    .await?;
    Ok(slot.unwrap_or(0))
}

async fn advance_cursor(db: &PgPool, cursor: &Cursor, slot: i64) -> Result<()> {
    sqlx::query(
        "insert into chain_sync_cursors (network, program_id, last_processed_slot) \
         values ($1, $2, $3) \
         on conflict (network, program_id) do update set \
             last_processed_slot = excluded.last_processed_slot \
         where excluded.last_processed_slot > chain_sync_cursors.last_processed_slot",
    )
    .bind(&cursor.network)
    .bind(&cursor.program_id)
    .bind(slot)
    .execute(db)
    .await?;
    Ok(())
}

async fn get_or_create_participant(db: &PgPool, wallet: &str) -> Result<Uuid> {
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

/// Runs one full pass: `commitment`/`page_size` distinguish the frequent
/// fast-path tick (spec §16's suggested `confirmed`, a small recent page)
/// from the periodic wider reconciliation pass (spec §16's "reconciliação
/// periódica" / "tratar reorganizações": re-scanning a larger page at
/// `finalized` catches anything a `confirmed`-level reorg or a missed tick
/// left behind — both call this same function, just with different
/// parameters, rather than duplicating the logic).
pub async fn run_pass(
    db: &PgPool,
    redis: &mut redis::aio::ConnectionManager,
    rpc: &RpcClient,
    cursor: &Cursor,
    page_size: usize,
    commitment: &str,
) -> Result<usize> {
    let program_address = &cursor.program_id;
    let cursor_slot = load_cursor_slot(db, cursor).await?;

    // Page backward (newest-first, via `before`) until we've either seen
    // every signature newer than the cursor, or run out of pages. Capped
    // at MAX_PAGES so a stale/never-set cursor against a long-lived
    // program can't turn one tick into an unbounded scan of chain history.
    const MAX_PAGES: u32 = 10;
    let per_page = page_size.min(PAGE_SIZE);
    let mut all_new = Vec::new();
    let mut before: Option<String> = None;
    for _ in 0..MAX_PAGES {
        let page = rpc
            .get_signatures_for_address(program_address, per_page, before.as_deref(), commitment)
            .await?;
        if page.is_empty() {
            break;
        }
        let mut hit_cursor = false;
        for info in &page {
            if info.slot as i64 <= cursor_slot {
                hit_cursor = true;
                break;
            }
            all_new.push(info.clone());
        }
        if hit_cursor || page.len() < per_page {
            break;
        }
        before = page.last().map(|info| info.signature.clone());
    }

    // Oldest-first, so the cursor only ever advances monotonically as
    // each one is persisted.
    all_new.reverse();

    let mut processed = 0usize;
    for info in all_new {
        if info.err.is_some() {
            // Failed transactions never reach the program logic that
            // would emit AttendanceRecorded — nothing to index, but the
            // cursor still needs to move past it.
            advance_cursor(db, cursor, info.slot as i64).await?;
            continue;
        }

        let Some(tx) = rpc.get_transaction(&info.signature, commitment).await? else {
            // Not yet visible at `confirmed` from this RPC node — leave
            // the cursor where it is; it'll be picked up on the next tick.
            break;
        };
        let Some(meta) = &tx.meta else { continue };
        if meta.err.is_some() {
            // Belt-and-suspenders: `info.err` (from getSignaturesForAddress,
            // checked above) should already have caught this, but a
            // transaction-level failure is never a source of
            // AttendanceRecorded either way.
            advance_cursor(db, cursor, info.slot as i64).await?;
            continue;
        }
        let Some(log_messages) = &meta.log_messages else {
            advance_cursor(db, cursor, info.slot as i64).await?;
            continue;
        };

        if let Some(event) = find_attendance_recorded(log_messages) {
            process_attendance(db, redis, &info.signature, tx.slot, tx.block_time, &event).await?;
        }

        advance_cursor(db, cursor, info.slot as i64).await?;
        processed += 1;
    }

    Ok(processed)
}

async fn process_attendance(
    db: &PgPool,
    redis: &mut redis::aio::ConnectionManager,
    signature: &str,
    slot: u64,
    block_time: Option<i64>,
    event: &eventquest_chain::events::AttendanceRecorded,
) -> Result<()> {
    let event_id: Option<Uuid> =
        sqlx::query_scalar("select id from events where onchain_event_address = $1")
            .bind(event.event.to_string())
            .fetch_optional(db)
            .await?;
    let checkpoint_id: Option<Uuid> =
        sqlx::query_scalar("select id from checkpoints where onchain_checkpoint_address = $1")
            .bind(event.checkpoint.to_string())
            .fetch_optional(db)
            .await?;

    // Unprovisioned off-chain rows (see apps/api/src/checkins/mod.rs's
    // header comment on the organizer on-chain-provisioning gap) mean
    // there's nowhere off-chain to attach this attendance yet — log and
    // move on rather than fail the whole pass; a later reconciliation
    // pass will pick it up once the mapping exists.
    let (Some(event_id), Some(checkpoint_id)) = (event_id, checkpoint_id) else {
        tracing::warn!(
            signature,
            event = %event.event,
            checkpoint = %event.checkpoint,
            "AttendanceRecorded for an event/checkpoint with no off-chain record yet — skipping"
        );
        return Ok(());
    };

    let participant_id = get_or_create_participant(db, &event.participant.to_string()).await?;
    let (attendance_pda, _) = pda::attendance(&event.event, &event.checkpoint, &event.participant);
    let block_time_ts = Utc
        .timestamp_opt(block_time.unwrap_or(event.checked_in_at), 0)
        .single()
        .unwrap_or_else(Utc::now);
    let challenge_hash = hex::encode(event.challenge_hash);

    let mut tx = db.begin().await?;

    let inserted: Option<Uuid> = sqlx::query_scalar(
        "insert into attendances ( \
             event_id, checkpoint_id, participant_id, onchain_attendance_address, \
             transaction_signature, block_time, slot, points_awarded, challenge_hash \
         ) values ($1, $2, $3, $4, $5, $6, $7, $8, $9) \
         on conflict do nothing \
         returning id",
    )
    .bind(event_id)
    .bind(checkpoint_id)
    .bind(participant_id)
    .bind(attendance_pda.to_string())
    .bind(signature)
    .bind(block_time_ts)
    .bind(slot as i64)
    .bind(event.points_awarded as i32)
    .bind(&challenge_hash)
    .fetch_optional(&mut *tx)
    .await?;

    if inserted.is_some() {
        sqlx::query(
            "insert into event_participants (event_id, participant_id, points, checkin_count) \
             values ($1, $2, $3, 1) \
             on conflict (event_id, participant_id) do update set \
                 points = event_participants.points + excluded.points, \
                 checkin_count = event_participants.checkin_count + 1",
        )
        .bind(event_id)
        .bind(participant_id)
        .bind(event.points_awarded as i64)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            "update checkin_attempts set status = 'confirmed' \
             where checkpoint_id = $1 and participant_id = $2 and status <> 'confirmed'",
        )
        .bind(checkpoint_id)
        .bind(participant_id)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    if inserted.is_some() {
        publish_attendance(
            redis,
            event_id,
            checkpoint_id,
            participant_id,
            event,
            signature,
        )
        .await;
    }

    Ok(())
}

/// spec §16 step 7 ("Publica atualização em tempo real"). Best-effort — a
/// missed publish never loses data (the row is already committed; a
/// polling client like `GET /check-ins/{grantId}` still converges on the
/// truth), so a publish failure is logged, not propagated.
async fn publish_attendance(
    redis: &mut redis::aio::ConnectionManager,
    event_id: Uuid,
    checkpoint_id: Uuid,
    participant_id: Uuid,
    event: &eventquest_chain::events::AttendanceRecorded,
    signature: &str,
) {
    let payload = serde_json::json!({
        "eventId": event_id,
        "checkpointId": checkpoint_id,
        "participantId": participant_id,
        "wallet": event.participant.to_string(),
        "pointsAwarded": event.points_awarded,
        "transactionSignature": signature,
        "checkedInAt": event.checked_in_at,
    });
    let result: redis::RedisResult<i64> = redis
        .publish("eventquest:attendance-events", payload.to_string())
        .await;
    if let Err(error) = result {
        tracing::warn!(%error, "failed to publish attendance update");
    }
}
