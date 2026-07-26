# Solana Program Security Audit

**Date**: 2026-07-26
**Program**: `eventquest` (`programs/eventquest`), deployed Devnet program ID `CuULC7T6d49mf89sRPU8WLeYfkF74kDyEKvfCe2rXq8H`
**Auditor**: Claude (self-audit, `/audit-solana`), verified by direct code review + `cargo clippy --all-targets -W clippy::all -D warnings` + `cargo audit` + full test suite, not tool output taken on faith. Re-run same day (see "Re-verification pass" below) after other branch work (infra/CI/frontend) landed — program code itself untouched.

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

## Re-verification pass — 2026-07-26 (same day, `/audit-solana` re-run)

`git log --oneline -- programs/eventquest crates/eventquest-chain` confirms **zero commits** to the on-chain program or its shared chain crate since this audit's original pass — all work on the branch since then (`/diff-review`, `/audit-infra`, the Docker/Railway deployment, wallet UX changes) touched `apps/api`, `apps/web`, CI, and infra config only. This pass exists to confirm nothing upstream regressed the program's security posture, not to redo the manual account/PDA/CPI review above, which stands unchanged.

- **`cargo fmt --check`**: clean.
- **`cargo audit`**: same 3 advisories as before (`rsa` via `jsonwebtoken`, plus the `mollusk-svm` dev-dependency transitive set) — `Cargo.lock` unchanged, still fully justified in `docs/SECURITY.md` SEC-04/SEC-05.
- **`cargo build-sbf` + `cargo test -p eventquest`**: 15 integration tests + 1 trivial lib unit test, all passing, same names/coverage as the "Testing Coverage" section above — no regression.
- **Stricter one-time pass**: `cargo clippy --all-targets -W clippy::all -W clippy::pedantic -W clippy::unwrap_used -W clippy::expect_used -W clippy::panic -W clippy::arithmetic_side_effects -D warnings` (this command's own Step 1, stricter than the project's normal CI lint of `clippy::all -D warnings`) surfaced findings across the whole workspace, not just the program. Reviewed each category that touched security-relevant code:
  - **`arithmetic_side_effects` in `check_in.rs:50`, `create_checkpoint.rs:24`, `initialize_event.rs:16`, `join_event.rs:26`**: all four are `space = T::DISCRIMINATOR.len() + T::INIT_SPACE` inside an `#[account(init, ...)]` attribute — a compile-time constant sum computed once per Anchor macro expansion, never touching user input, never capable of overflowing. **False positive**, consistent with the "Arithmetic Security" section's existing `checked_add` audit of the program's actual runtime counters.
  - **`arithmetic_side_effects` in `apps/api`/`apps/indexer`** (`process.rs:161` a bounded per-batch loop counter, `rpc.rs:78` an exponential-backoff delay bounded by a small `MAX_ATTEMPTS`, `main.rs:77` a `u32` polling-tick counter that would take over a century to wrap at any realistic interval, `auth/handlers.rs:38` and `blockchain/client.rs:137` date/duration arithmetic over constants or internally-supplied values, never attacker-controlled input) — reviewed individually, all outside the on-chain program's own attack surface and none reachable with attacker-influenced operands. **False positives**, not added to `docs/SECURITY.md` since none rises to a documented-deferral bar (no plausible exploit scenario).
  - **`unwrap_used`/`expect_used` (3 sites: `crates/eventquest-domain/src/error.rs:171,179`, `crates/eventquest-config/src/lib.rs:240`)**: confirmed via `grep -n "mod tests"` that all three sites are inside `#[cfg(test)] mod tests` blocks — allowed per this project's own `rust.md` ("`unwrap()` is acceptable in tests"). **Not findings.**
  - Remaining pedantic noise (`must_use_candidate`, `missing_errors_doc`, doc-backtick formatting, an unused `use *` wildcard import, a `u32`-to-`u64` cast expressible via `From`, two identical `match` arms) is pure style/lint preference, not security-relevant, and out of scope for a security audit — not pursued further (this project's actual CI gate is `clippy::all -D warnings`, which already passes clean).

**Conclusion**: no new findings. The program's security posture is unchanged from the original pass above.
