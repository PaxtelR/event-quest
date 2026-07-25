# ADR-001: Hybrid on-chain/off-chain architecture for EventQuest

## Status

Accepted

## Context

EventQuest needs to prove that a specific wallet was present at a specific
checkpoint during a specific time window, without putting personal data on a
public, permanent ledger, and without depending on the frontend or a single
backend request/response cycle as the source of truth for "attendance
happened."

The full functional and technical specification
(`EventQuest_Especificacao_IA_Solana_AI_Kit_EN_Dark.md`) mandates:

- Only identifiers, timestamps, hashes, and points on-chain (§3);
- No personal data (name, email, CPF, IP, precise geolocation, device ids) on
  the blockchain (§3, §19);
- Solana Devnet only for the MVP, guarded against accidental Mainnet
  deployment (§0.7, §12.1);
- Rust/Axum/Postgres/Redis for the API, not a second API in Next.js route
  handlers (§12.1);
- An indexer as the final source of truth for confirmed attendance, not the
  browser's callback after sending a transaction (§16, §28 item 19-20).

## Decision

Adopt a hybrid architecture with a hard boundary between what is on-chain and
what is off-chain, matching spec §3 exactly:

**On-chain (Anchor program `eventquest_program`, Solana Devnet):**
- `EventAccount`, `CheckpointAccount`, `ParticipantEventAccount`,
  `AttendanceAccount` — identifiers (as pubkeys / 32-byte hashes of external
  ids), timestamps, points, counters, status enums.
- The `check_in` instruction requires two signers: the participant and the
  checkpoint's configured `attestor`. Points are always read from
  `CheckpointAccount`, never taken from instruction arguments supplied by a
  client.
- An `AttendanceRecorded` event is emitted on every successful check-in; this
  is the only channel the indexer trusts.

**Off-chain (Postgres via `apps/api`, Redis, `apps/indexer`):**
- Human-readable event/checkpoint metadata (name, description, images,
  address), participant profile fields, missions, reports, audit logs.
- QR rotation windows and nonces, check-in grants, sessions, rate limits — all
  ephemeral, all in Redis, none of it a system of record.
- `apps/indexer` is a separate worker process from `apps/api`. It is the only
  writer of the `attendances` table's on-chain-derived fields (signature,
  slot, block time, PDA address). The API's own check-in flow may optimistically
  mark a `checkin_attempts` row as `TRANSACTION_SUBMITTED`, but attendance is
  only considered `CONFIRMED` once the indexer has independently observed and
  persisted the `AttendanceRecorded` event for that PDA. This satisfies spec
  §28 item 19 ("do not rely solely on the browser callback") and item 20
  ("do not mark attendance before on-chain confirmation").

**Privacy rule:** the participant's public key is the on-chain identifier.
Nothing in spec §3's forbidden list (name, CPF, email, phone, address, IP,
precise geolocation, device identifiers, personal documents) is ever written
to an instruction argument or account field.

**Network rule:** `apps/api` and `apps/indexer` refuse to start unless
`SOLANA_NETWORK=devnet` (spec §12.1). `scripts/verify-devnet-config.sh`
(added in the Devnet-deployment phase) independently fails the build/deploy
pipeline if it detects any Mainnet RPC URL, `Anchor.toml` cluster, or program
manifest mismatch, so a single misconfigured environment variable cannot
result in an accidental Mainnet deployment.

## Consequences

- Every account and instruction addition must be re-justified against the
  on-chain/off-chain split above — "would this data element need to be
  auditable independent of EventQuest's own database" is the test for
  putting something on-chain (spec §3).
- The indexer's cursor (`chain_sync_cursors` table) and idempotent upsert
  logic are load-bearing for correctness, not an optimization — restart
  safety and duplicate-free reprocessing are required from the first version,
  not deferred.
- The API can be scaled/restarted freely without risking double-counted
  attendance, because attendance state ultimately comes from the chain via
  the indexer, not from in-memory or Redis state.
