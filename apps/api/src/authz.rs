//! Organization-membership authorization, shared by the events and
//! checkpoints modules (spec §5.2: organizer capabilities are scoped to the
//! organizations a wallet belongs to).

use eventquest_domain::ErrorCode;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::ApiError;

/// Confirms `wallet` is an `owner` or `admin` of `organization_id`. The
/// `operator` role (spec §5.3, scoped to checkpoint display/activation
/// only) is checked separately by callers that allow it.
pub async fn require_org_admin(
    db: &PgPool,
    organization_id: Uuid,
    wallet: &str,
) -> Result<(), ApiError> {
    let role: Option<String> = sqlx::query_scalar(
        "select role::text from organization_members \
         where organization_id = $1 and wallet_address = $2",
    )
    .bind(organization_id)
    .bind(wallet)
    .fetch_optional(db)
    .await?;

    match role.as_deref() {
        Some("owner") | Some("admin") => Ok(()),
        _ => Err(ApiError::new(ErrorCode::Forbidden)),
    }
}

/// Like `require_org_admin`, but also accepts the `operator` role — for
/// actions checkpoint operators are allowed to perform (activate/pause
/// their own checkpoint) without full event-admin rights.
pub async fn require_org_member(
    db: &PgPool,
    organization_id: Uuid,
    wallet: &str,
) -> Result<(), ApiError> {
    let role: Option<String> = sqlx::query_scalar(
        "select role::text from organization_members \
         where organization_id = $1 and wallet_address = $2",
    )
    .bind(organization_id)
    .bind(wallet)
    .fetch_optional(db)
    .await?;

    match role.as_deref() {
        Some("owner") | Some("admin") | Some("operator") => Ok(()),
        _ => Err(ApiError::new(ErrorCode::Forbidden)),
    }
}

/// Looks up the organization an event belongs to. `ResourceNotFound` if the
/// event doesn't exist — callers combine this with an admin/member check.
pub async fn event_organization_id(db: &PgPool, event_id: Uuid) -> Result<Uuid, ApiError> {
    sqlx::query_scalar("select organization_id from events where id = $1")
        .bind(event_id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| ApiError::new(ErrorCode::ResourceNotFound))
}

/// Looks up the organization a checkpoint's event belongs to.
pub async fn checkpoint_organization_id(
    db: &PgPool,
    checkpoint_id: Uuid,
) -> Result<Uuid, ApiError> {
    sqlx::query_scalar(
        "select e.organization_id from checkpoints c \
         join events e on e.id = c.event_id \
         where c.id = $1",
    )
    .bind(checkpoint_id)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| ApiError::new(ErrorCode::ResourceNotFound))
}

/// Ensures `wallet` belongs to at least one organization, auto-provisioning
/// a personal one if not. Spec §12.3 lists event/checkpoint endpoints but
/// never an explicit "create organization" endpoint — this fills that gap
/// pragmatically (see apps/api/src/events/mod.rs for the fuller rationale)
/// rather than blocking the vertical slice on an unspecified flow.
pub async fn ensure_personal_organization(db: &PgPool, wallet: &str) -> Result<Uuid, ApiError> {
    if let Some(existing) = sqlx::query_scalar::<_, Uuid>(
        "select organization_id from organization_members \
         where wallet_address = $1 order by created_at asc limit 1",
    )
    .bind(wallet)
    .fetch_optional(db)
    .await?
    {
        return Ok(existing);
    }

    let mut tx = db.begin().await?;

    let short_wallet = wallet.chars().take(8).collect::<String>();
    let name = format!("{short_wallet}'s workspace");
    let slug = format!("wallet-{}", wallet.to_lowercase());

    let organization_id: Uuid =
        sqlx::query_scalar("insert into organizations (name, slug) values ($1, $2) returning id")
            .bind(&name)
            .bind(&slug)
            .fetch_one(&mut *tx)
            .await?;

    sqlx::query(
        "insert into organization_members (organization_id, wallet_address, role) \
         values ($1, $2, 'owner')",
    )
    .bind(organization_id)
    .bind(wallet)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(organization_id)
}
