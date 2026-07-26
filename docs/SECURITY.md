# Security — EventQuest

Consolidated record of every finding raised during `/audit-solana` and
`/audit-infra` (2026-07-26) that was **not** fixed immediately, per spec
§25.7. Findings that were fixed in the same pass are documented in
`docs/audits/infra-2026-07-26.md` and `docs/security-audit-2026-07-26.md`
instead — they don't need a deferral entry here.

For the full audit methodology and everything that was checked and found
clean, see:
- `docs/security-audit-2026-07-26.md` — on-chain program audit
- `docs/audits/infra-2026-07-26.md` — infrastructure audit

## SEC-01: `update_event_status` allows any non-terminal → non-terminal transition

**Severity**: Low
**Evidence**: `programs/eventquest/src/instructions/update_event_status.rs`'s
handler only checks `!event.status.is_terminal()` before applying
`new_status` — there's no explicit state machine preventing e.g.
`Active → Draft` or `Paused → Draft`, only the two truly one-way gates
(`Finished`/`Cancelled` can never transition again).
**Impact**: Only the event's own `authority` (organizer, enforced by
`has_one = authority`) can call this instruction — there's no path for a
participant or third party to exploit this. The worst case is an organizer
confusing their own event's lifecycle (e.g. accidentally moving an Active
event back to Draft), which is a UX footgun, not a security boundary
violation.
**Mitigation**: `apps/api`'s `events::handlers` only ever requests the
transitions the UI exposes (`publish`, `pause`, `finish`) — the looser
on-chain instruction is a superset of what the application actually drives.
**Justification for deferral**: Not a security vulnerability — an
authority-only self-service action. A stricter on-chain transition table
would need a program upgrade + redeploy for marginal benefit; revisit if a
future multi-admin organization model makes an organizer's mistake affect
other organizers' visibility into the same event.

## SEC-02: `checkpoint.attestor` accepted without sanity validation

**Severity**: Low
**Evidence**: `create_checkpoint`/`update_checkpoint` accept any `Pubkey`
for `attestor`, including in principle the zero pubkey or the System
Program's address.
**Impact**: Self-inflicted only. If an organizer sets an attestor they
don't hold the key for, `check_in` simply becomes permanently unsatisfiable
for that checkpoint (`InvalidAttestor` on every attempt) until the
organizer calls `update_checkpoint` again with a real attestor pubkey — no
funds or other participants' data are at risk.
**Mitigation**: `apps/api`'s checkpoint-creation flow always supplies the
backend's own loaded attestor pubkey (`state::load_attestor_pubkey`), so
the application's actual UI path can't produce this state; it's only
reachable via direct program interaction (as the smoke test does
deliberately, using a real, working attestor keypair).
**Justification for deferral**: Adding an on-chain non-zero/non-system
check would cost a small amount of CU on every `create_checkpoint`/
`update_checkpoint` call to guard against a mistake only the calling
organizer can make against themselves. Not worth the tradeoff for a
Devnet MVP; revisit if third-party integrators start calling the program
directly instead of through `apps/api`.

## SEC-03: Trident fuzz testing not run

**Severity**: Low (informational — this is a checklist item, not a
discovered vulnerability)
**Evidence**: `/audit-solana`'s Step 9 requires "REQUIRED for mainnet"
fuzz testing via Trident; none was run this pass.
**Impact**: Unknown — fuzzing might surface edge cases the deterministic
Mollusk test suite (15 cases covering every negative path listed in spec
§25.1) doesn't hit, but no specific gap has been identified.
**Mitigation**: The deterministic test suite already covers every
documented negative case (duplicate check-in, wrong attestor, inactive
event/checkpoint, cross-event PDA confusion, points-locked-after-checkins,
arithmetic bounds) plus four live-Devnet integration tests.
**Justification for deferral**: Trident fuzzing is explicitly a
pre-**Mainnet** gate in the kit's own audit checklist ("REQUIRED for
mainnet"), and this project is Devnet-only by spec mandate (§0.7). Required
before any future Mainnet consideration, not before Devnet MVP completion.

## SEC-04: `rsa` crate (RUSTSEC-2023-0071) — transitive, unreachable code path

**Severity**: Low (upstream-reported as Medium/5.9, downgraded here on
reachability grounds — see evidence)
**Evidence**: `cargo audit` flags `rsa` 0.9.10, pulled in transitively via
`jsonwebtoken` 11.0.0 (`apps/api`'s direct dependency, used with
`default-features = false, features = ["rust_crypto"]`). `apps/api`'s only
two `jsonwebtoken` call sites (`apps/api/src/qr/claims.rs`) exclusively use
`Algorithm::HS256` — confirmed by `grep -rn "Algorithm::" apps/api/src/`.
**Impact**: The vulnerable code (RSA private-key timing side-channel
during signing/decryption) is compiled into the binary but never executed
by this application — HS256 is HMAC-based and never touches `rsa`'s code
paths.
**Mitigation**: None needed at the application level; the exposure is
purely "present in the dependency tree," not "reachable."
**Justification for deferral**: No upgrade path exists yet (`cargo audit`
reports "No fixed upgrade is available" for this advisory), and the code
path is unreachable regardless. Re-check on every `jsonwebtoken`/`rsa`
version bump. Allow-listed in `.cargo/audit.toml` (added via `/setup-ci-cd`)
so CI's `cargo audit` step fails on any *new* advisory without re-flagging
this one — remove the ignore entry once a fix ships upstream.

## SEC-05: Several `mollusk-svm` dev-dependency advisories

**Severity**: Low (informational)
**Evidence**: `curve25519-dalek` 3.2.0 (RUSTSEC-2024-0344), `ed25519-dalek`
1.0.1 (RUSTSEC-2022-0093), `rand` 0.7.3 (RUSTSEC-2026-0097), plus
unmaintained warnings for `atty`/`bincode`/`derivative`/`libsecp256k1`/
`paste`. `cargo tree -i` on each confirms every one of these resolves
exclusively through `mollusk-svm` (the on-chain program's `[dev-dependencies]`
test harness) or `anchor-lang`'s own internal, non-swappable dependency
graph.
**Impact**: None reachable in production — `[dev-dependencies]` are never
compiled into the deployed program's `.so` or into `apps/api`/
`apps/indexer`'s release binaries; they only exist inside `cargo test`
processes on developer/CI machines.
**Mitigation**: None needed — not part of the attack surface.
**Justification for deferral**: Upgrading these would mean upgrading
`mollusk-svm` or `anchor-lang` itself, both pinned deliberately to match
the installed `anchor-cli 1.1.2`/Solana 3.x toolchain (see the extensive
version-pinning history in this project's ADRs and `.claude/rules/anchor.md`).
Revisit when the next Anchor/Mollusk release naturally picks up newer
transitive versions. The two advisories with RUSTSEC IDs
(`curve25519-dalek`/`ed25519-dalek`) are allow-listed in `.cargo/audit.toml`;
the remaining unmaintained-package warnings (`atty`, `bincode`,
`derivative`, `libsecp256k1`, `paste`) don't fail `cargo audit` by default
and need no entry.

## SEC-06 (resolved): `docker-compose.yml` referenced nonexistent Dockerfiles

Was: `infrastructure/docker/{api,indexer,web}.Dockerfile` didn't exist,
even though `docker-compose.yml` referenced them. Resolved — all three
now exist, are built and run-tested (including a real container-to-
container run against Postgres/Redis), and are deployed to production
(Railway) behind `eventquest.paxtel.com.br`. See the README's
"Deployment" section for the two real bugs that surfaced only once
actually deployed (build-time vs. runtime env vars, and the container
`HOSTNAME` variable breaking Next's standalone server's bind address)
and how they were fixed.

## SEC-07: No rate limiting on `/auth/nonce` and `/auth/verify`

**Severity**: Low
**Evidence**: `checkins::handlers::check_rate_limit` is applied to the
check-in flow only; `apps/api/src/auth/handlers.rs`'s `nonce`/`verify`
handlers have no equivalent.
**Impact**: An attacker could request many nonces cheaply (each is a
16-byte random token stored in Redis with a 5-minute TTL) — a minor
Redis-memory nuisance, capped naturally by the TTL. `verify` requires a
valid ed25519 signature over a specific nonce-bound message, which is
itself expensive for an attacker to produce at volume and doesn't
succeed without a real wallet's private key.
**Mitigation**: Redis TTL bounds nonce accumulation; signature
verification bounds `verify` abuse.
**Justification for deferral**: Lower practical risk than the check-in
flow (which directly gates points/attendance state), and the existing
TTL + signature-verification cost already provide meaningful friction.
Add alongside any broader API rate-limiting pass (e.g. a `tower-governor`
layer applied uniformly) rather than a one-off fix here.
