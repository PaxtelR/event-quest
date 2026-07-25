-- Initial off-chain schema for EventQuest — see
-- EventQuest_Especificacao_IA_Solana_AI_Kit_EN_Dark.md §13 and §3 (only
-- human-readable/mutable data lives here; identifiers, timestamps, hashes,
-- and points that need to be publicly auditable live on Solana instead —
-- see docs/adr/ADR-001-architecture.md).

create extension if not exists pgcrypto;

-- Shared trigger to keep `updated_at` correct without relying on every
-- call site remembering to set it.
create or replace function set_updated_at()
returns trigger as $$
begin
  new.updated_at = now();
  return new;
end;
$$ language plpgsql;

-- ---------------------------------------------------------------------------
-- organizations / organization_members
-- ---------------------------------------------------------------------------

create table organizations (
  id uuid primary key default gen_random_uuid(),
  name varchar not null,
  slug varchar not null unique,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create trigger organizations_set_updated_at
  before update on organizations
  for each row execute function set_updated_at();

-- `owner` created the organization; `admin` manages events end to end;
-- `operator` is scoped to running checkpoint displays (spec §5.3: cannot
-- edit the whole event unless also an admin/owner).
create type organization_role as enum ('owner', 'admin', 'operator');

create table organization_members (
  organization_id uuid not null references organizations(id) on delete cascade,
  wallet_address varchar not null,
  role organization_role not null,
  created_at timestamptz not null default now(),
  primary key (organization_id, wallet_address)
);

-- ---------------------------------------------------------------------------
-- events
-- ---------------------------------------------------------------------------

-- Mirrors the on-chain EventStatus enum (programs/eventquest/src/state.rs)
-- so the off-chain projection can't drift into an unrepresentable state.
create type event_status as enum ('draft', 'active', 'paused', 'finished', 'cancelled');
create type event_visibility as enum ('public', 'private');
create type solana_network as enum ('devnet', 'mainnet');

create table events (
  id uuid primary key default gen_random_uuid(),
  organization_id uuid not null references organizations(id) on delete cascade,
  name varchar not null,
  slug varchar not null,
  description text,
  banner_url text,
  location_name varchar,
  location_address text,
  timezone varchar not null default 'UTC',
  starts_at timestamptz not null,
  ends_at timestamptz not null,
  status event_status not null default 'draft',
  visibility event_visibility not null default 'private',
  solana_network solana_network not null default 'devnet',
  onchain_event_address varchar,
  onchain_create_signature varchar,
  created_by_wallet varchar not null,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  constraint events_period_valid check (starts_at < ends_at),
  constraint events_slug_per_org_unique unique (organization_id, slug)
);

create trigger events_set_updated_at
  before update on events
  for each row execute function set_updated_at();

create index events_organization_id_idx on events(organization_id);
create index events_status_idx on events(status);

-- ---------------------------------------------------------------------------
-- checkpoints
-- ---------------------------------------------------------------------------

create type checkpoint_status as enum ('draft', 'active', 'paused', 'closed');

create table checkpoints (
  id uuid primary key default gen_random_uuid(),
  event_id uuid not null references events(id) on delete cascade,
  name varchar not null,
  description text,
  points integer not null,
  rotation_seconds integer not null default 15,
  opens_at timestamptz not null,
  closes_at timestamptz not null,
  status checkpoint_status not null default 'draft',
  attestor_pubkey varchar not null,
  onchain_checkpoint_address varchar,
  display_token_hash varchar,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  constraint checkpoints_period_valid check (opens_at < closes_at),
  constraint checkpoints_points_positive check (points > 0),
  constraint checkpoints_rotation_seconds_bounds
    check (rotation_seconds between 10 and 60)
);

create trigger checkpoints_set_updated_at
  before update on checkpoints
  for each row execute function set_updated_at();

create index checkpoints_event_id_idx on checkpoints(event_id);
create index checkpoints_status_idx on checkpoints(status);

-- ---------------------------------------------------------------------------
-- participants / event_participants
-- ---------------------------------------------------------------------------

create table participants (
  id uuid primary key default gen_random_uuid(),
  wallet_address varchar not null unique,
  display_name varchar,
  email varchar,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create trigger participants_set_updated_at
  before update on participants
  for each row execute function set_updated_at();

-- `active`: currently part of the event. `removed`: organizer revoked
-- participation (e.g. abuse) without deleting check-in history.
create type event_participant_status as enum ('active', 'removed');

create table event_participants (
  event_id uuid not null references events(id) on delete cascade,
  participant_id uuid not null references participants(id) on delete cascade,
  status event_participant_status not null default 'active',
  points bigint not null default 0,
  checkin_count integer not null default 0,
  completed boolean not null default false,
  joined_at timestamptz not null default now(),
  completed_at timestamptz,
  primary key (event_id, participant_id),
  constraint event_participants_points_non_negative check (points >= 0),
  constraint event_participants_checkin_count_non_negative check (checkin_count >= 0)
);

create index event_participants_participant_id_idx on event_participants(participant_id);

-- ---------------------------------------------------------------------------
-- checkin_attempts — off-chain state machine from spec §9.5
-- ---------------------------------------------------------------------------

create type checkin_attempt_status as enum (
  'qr_validated',
  'transaction_prepared',
  'transaction_submitted',
  'confirmed',
  'failed',
  'expired',
  'rejected'
);

create table checkin_attempts (
  id uuid primary key default gen_random_uuid(),
  grant_id uuid not null unique,
  event_id uuid not null references events(id) on delete cascade,
  checkpoint_id uuid not null references checkpoints(id) on delete cascade,
  participant_id uuid not null references participants(id) on delete cascade,
  qr_jti_hash varchar not null,
  challenge_hash varchar not null,
  idempotency_key varchar not null,
  status checkin_attempt_status not null default 'qr_validated',
  transaction_signature varchar,
  failure_code varchar,
  failure_message text,
  expires_at timestamptz not null,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  -- One in-flight/completed attempt per (checkpoint, participant) — the
  -- application still relies on the on-chain Attendance PDA as the ultimate
  -- duplicate-check-in guard (spec §11.2), this is the off-chain mirror.
  constraint checkin_attempts_checkpoint_participant_unique unique (checkpoint_id, participant_id)
);

create trigger checkin_attempts_set_updated_at
  before update on checkin_attempts
  for each row execute function set_updated_at();

create index checkin_attempts_participant_id_idx on checkin_attempts(participant_id);
create index checkin_attempts_idempotency_key_idx on checkin_attempts(idempotency_key);

-- ---------------------------------------------------------------------------
-- attendances — populated exclusively by apps/indexer from on-chain
-- AttendanceRecorded events (spec §16); never written directly by apps/api.
-- ---------------------------------------------------------------------------

create table attendances (
  id uuid primary key default gen_random_uuid(),
  event_id uuid not null references events(id) on delete cascade,
  checkpoint_id uuid not null references checkpoints(id) on delete cascade,
  participant_id uuid not null references participants(id) on delete cascade,
  onchain_attendance_address varchar not null unique,
  transaction_signature varchar not null unique,
  block_time timestamptz not null,
  slot bigint not null,
  points_awarded integer not null,
  challenge_hash varchar not null,
  confirmed_at timestamptz not null default now(),
  created_at timestamptz not null default now(),
  constraint attendances_checkpoint_participant_unique unique (checkpoint_id, participant_id),
  constraint attendances_points_non_negative check (points_awarded >= 0)
);

create index attendances_event_id_idx on attendances(event_id);
create index attendances_participant_id_idx on attendances(participant_id);

-- ---------------------------------------------------------------------------
-- chain_sync_cursors — apps/indexer restart-safety (spec §16)
-- ---------------------------------------------------------------------------

create table chain_sync_cursors (
  network varchar not null,
  program_id varchar not null,
  last_processed_slot bigint not null default 0,
  updated_at timestamptz not null default now(),
  primary key (network, program_id)
);

create trigger chain_sync_cursors_set_updated_at
  before update on chain_sync_cursors
  for each row execute function set_updated_at();

-- ---------------------------------------------------------------------------
-- audit_logs — spec §13 (no plaintext IP, per spec §13 note and §19.5)
-- ---------------------------------------------------------------------------

create table audit_logs (
  id uuid primary key default gen_random_uuid(),
  actor_wallet varchar,
  action varchar not null,
  resource_type varchar not null,
  resource_id uuid,
  metadata jsonb not null default '{}'::jsonb,
  ip_hash varchar,
  created_at timestamptz not null default now()
);

create index audit_logs_actor_wallet_idx on audit_logs(actor_wallet);
create index audit_logs_resource_idx on audit_logs(resource_type, resource_id);
create index audit_logs_created_at_idx on audit_logs(created_at);
