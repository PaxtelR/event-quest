# Solana Program Security Audit

**Date**: 2026-07-26
**Program**: `eventquest` (`programs/eventquest`), deployed Devnet program ID `CuULC7T6d49mf89sRPU8WLeYfkF74kDyEKvfCe2rXq8H`
**Auditor**: Claude (self-audit, `/audit-solana`), verified by direct code review + `cargo clippy --all-targets -W clippy::all -D warnings` + `cargo audit` + full test suite, not tool output taken on faith

## Summary

| Severity | Count |
|---|---|
| Critical | 0 |
| High | 0 |
| Medium | 0 |
| Low | 2 (documented in `docs/SECURITY.md`) |

No critical or high findings. Both low findings are design observations, not exploitable bugs — see `docs/SECURITY.md` for full writeups (`SEC-01`, `SEC-02`).

## Methodology

Every instruction handler (`initialize_event`, `update_event_status`, `create_checkpoint`, `update_checkpoint`, `join_event`, `check_in`, `finish_participant_event`) was read against the checklist below; findings were verified by grep across `programs/eventquest/src/` rather than assumed from memory.

## Account Security

- **Owner/signer checks**: every mutating instruction requires the correct `Signer<'info>`; `Account<'info, T>` is used throughout (never raw `AccountInfo` casts), so Anchor's discriminator check runs on every account load — no type-cosplay surface.
- **PDA validation**: all seven PDA-bearing accounts (`event`, `checkpoint`, `participant_event`, `attendance`, all four) use `seeds = [...]` + `bump = <account>.bump` against a **stored canonical bump** for every non-`init` access. `grep -rn "find_program_address" programs/eventquest/src/` returns nothing — no bump is ever recalculated at runtime. Every `init` handler stores `ctx.bumps.<account>` immediately (`event.bump`, `checkpoint.bump`, `participant_event.bump`, `attendance.bump`).
- **Seed collision**: `EVENT_SEED`/`CHECKPOINT_SEED`/`PARTICIPANT_SEED`/`ATTENDANCE_SEED` are four distinct byte-string prefixes (`constants.rs`) — no shared PDA space.
- **Duplicate check-in prevention**: `attendance` uses `init` (never `init_if_needed`) on seeds unique to `(event, checkpoint, participant)` — a second `check_in` for the same triple fails at Solana's account-allocation layer ("already in use") before any handler logic runs, independent of what `challenge_hash` is passed. Verified live: `check_in_rejects_duplicate_attempt` (Mollusk) and two separate real-Devnet runs (`scripts/devnet-smoke-test.ts`, `apps/web/tests/e2e-devnet/`) both produced a genuine on-chain rejection.
- **Account revival**: not applicable — no instruction ever closes an account (`close = ...` is not used anywhere in this program), so there's no closed-account-revival surface to defend.

## Arithmetic Security

Every counter increment (`participant_event.points`, `participant_event.checkin_count`, `checkpoint.total_checkins`, `event.total_checkins`, `event.checkpoint_count`) uses `checked_add(...).ok_or(EventQuestError::Overflow)?`. `grep -rn "unwrap()\|\.expect(" programs/eventquest/src/` returns nothing. A regex scan for bare `+`/`-` operators outside `checked_*` calls returns no real matches (only false positives from hyphenated words inside `#[msg("...")]` strings, manually verified). Points are `u32`, capped at `MAX_POINTS_PER_CHECKPOINT = 10_000` per checkpoint, summed into a `u64` participant total — headroom against overflow is enormous even at scale.

## CPI Security

The only CPI in this program is Anchor's own internal System Program account creation via `#[account(init, ...)]`, validated implicitly by the `Program<'info, System>` type. There is no manual `invoke`/`invoke_signed` anywhere in `programs/eventquest/src/` — no arbitrary-CPI surface, nothing to reload after a CPI.

## Economic / Trust Logic

- **Points never trusted from the client**: `check_in_handler` reads `points` exclusively from `ctx.accounts.checkpoint.points` (spec §11.5); the instruction's only client-supplied argument affecting state is `challenge_hash`, which does not influence points.
- **Attestor identity**: `check_in` requires `ctx.accounts.attestor.key() == ctx.accounts.checkpoint.attestor` (`require_keys_eq!`), so only the checkpoint's configured co-signer can authorize a check-in — a participant cannot self-authorize.
- **Retroactive point changes blocked**: `update_checkpoint_handler` refuses to change `points` once `checkpoint.total_checkins > 0` (`CheckpointPointsLocked`) — already-awarded points can never be changed after the fact.
- **Time-window enforcement**: `check_in` requires `checkpoint.opens_at <= now <= checkpoint.closes_at` (on-chain `Clock`, not client-supplied time) in addition to `event.status == Active` and `checkpoint.active`.

## Error Handling

No `unwrap()`/`expect()`/`panic!` anywhere in `programs/eventquest/src/instructions/` or `state.rs`. All 13 error variants in `errors.rs` have descriptive `#[msg(...)]` text and are used precisely (verified each `require!`/`require_keys_eq!` site maps to a distinct, correctly-named variant).

## CU Discipline

Zero `msg!()` calls anywhere in the program (verified by grep) — no CU spent on logging in the hot path. Observed CU consumption from live test runs: `check_in` ≈ 10.9k–21.3k CU, `create_checkpoint`/`initialize_event` single-digit-thousands — all far below the 200k default budget, no compute-budget instruction needed.

## Testing Coverage

Mollusk unit tests (`cargo test -p eventquest`, 15 tests) cover: happy-path check-in, event-not-active rejection, checkpoint-not-active rejection, duplicate check-in rejection, points-locked-after-checkins rejection, checkpoint-from-another-event rejection (cross-event PDA confusion), and more. Four additional `#[ignore]`d tests exercise the identical flows against **real Solana Devnet** (`apps/api`'s `checkins::devnet_tests`, `organizer_devnet_tests`, `blockchain::client::devnet_tests`; `apps/indexer`'s `devnet_tests`) — run via `cargo test -- --ignored` with a funded attestor keypair, all passing as of this audit. Fuzz testing (Trident) was not run — see `docs/SECURITY.md` `SEC-03` for the deferral justification.

## Verifiable Build

Not run in this pass — `anchor build --verifiable` requires Docker and is a Mainnet-deployment gate per this command's own checklist; out of scope for a Devnet-only MVP. Noted as a pre-Mainnet action item, not a Devnet-MVP blocker.

## Sign-off

- [x] All critical and high issues: none found
- [x] Comprehensive automated tests passing (Mollusk + live Devnet)
- [ ] Professional third-party audit — not done (expected; this is a Devnet MVP, not a Mainnet-bound financial program)
- [ ] Fuzz testing (Trident) — deferred, see `docs/SECURITY.md` SEC-03
