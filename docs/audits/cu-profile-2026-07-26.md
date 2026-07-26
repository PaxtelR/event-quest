# CU Profile — EventQuest — 2026-07-26

**Program**: `eventquest` (`programs/eventquest`), Anchor 1.1.2, binary `target/deploy/eventquest.so` (257,843 bytes)
**Method**: Mollusk (in-process SVM, `mollusk-svm` 0.5.1) — no `anchor test`/local validator, per this project's actual test harness (`.claude/rules/anchor.md`: "Default test template is LiteSVM (Rust)"; this project uses Mollusk, the closest equivalent already in place). Numbers are real `compute_units_consumed` values from `mollusk_svm::result::InstructionResult`, not estimated or quoted from memory.

No CU logging existed anywhere in this repo before this pass — the earlier security audit's "check_in ≈ 10.9k–21.3k CU" figure was a rough range from ad hoc observation during Phase 6 Devnet testing, not a repeatable measurement. This profile replaces that with a single deterministic source of truth: `programs/eventquest/tests/integration.rs::cu_profile_prints_every_instructions_compute_units`, a new test that runs the full happy path for every instruction once (via the existing `World`/Mollusk harness already used by the other 15 tests) and prints each instruction's CU cost. Run it yourself any time with:

```bash
cargo build-sbf --manifest-path programs/eventquest/Cargo.toml
cargo test --manifest-path programs/eventquest/Cargo.toml --test integration cu_profile -- --nocapture
```

## Results

| Instruction | CU consumed | % of 200k default budget | Status |
|---|---|---|---|
| `check_in` | 19,406 | 9.7% | Efficient |
| `create_checkpoint` | 16,540 | 8.3% | Efficient |
| `join_event` | 10,902 | 5.5% | Efficient |
| `initialize_event` | 10,081 | 5.0% | Efficient |
| `update_checkpoint` | 6,452 | 3.2% | Efficient |
| `finish_participant_event` | 6,424 | 3.2% | Efficient |
| `update_event_status` | 4,002 | 2.0% | Efficient |

Every instruction falls in the "< 50,000 CU — Efficient" band by a wide margin (this command's own threshold table). None come close to needing a `ComputeBudgetProgram.setComputeUnitLimit` instruction above Solana's 200k default, and `apps/api`'s TypeScript client (per `.claude/rules/typescript.md`'s simulate-then-set-compute-budget pattern) already sizes the budget from simulation regardless.

## Why these numbers are already close to the floor

- **Zero `msg!()` calls** anywhere in `programs/eventquest/src/` (verified by grep, also noted in `docs/security-audit-2026-07-26.md`) — no logging tax (~100 CU/call) anywhere in the hot path.
- **No CPIs except Anchor's own internal System Program account creation** (`#[account(init, ...)]` on `event`/`checkpoint`/`participant_event`/`attendance`) — this is the single real cost driver visible in the numbers: every instruction with an `init`'d account (`initialize_event`, `create_checkpoint`, `join_event`, `check_in`) costs roughly 10k–19k CU, while the three mutate-only instructions with no `init` (`update_event_status`, `update_checkpoint`, `finish_participant_event`) cost only 4k–6.5k CU — consistent with account creation (rent calculation + System Program CPI + zero-initializing account data) being the dominant expense, not handler logic.
- **PDA bumps are stored and reused**, never recalculated (`grep -rn "find_program_address" programs/eventquest/src/` returns nothing) — every non-`init` account access uses `bump = <account>.bump` against a canonical stored bump (~200 CU) instead of `find_program_address`'s on-chain search (~1,500 CU per call, per this command's own reference table). This alone is why `update_checkpoint`/`finish_participant_event` (two PDA reads each) stay under 6.5k CU rather than costing several thousand more.
- **`check_in` is the most expensive instruction** (19,406 CU) because it's the only one touching five accounts including two `Account<'info, T>` reads with `has_one`/`require_keys_eq!` checks (`event`, `checkpoint`, `participant_event`) plus a new `attendance` PDA `init` — proportional to its account count, not to any inefficiency.

## Optimization opportunities considered, and why none are warranted

Per this command's own guidance (zero-copy `AccountLoader`, Pinocchio migration, batching): none of these are worth applying here. `AccountLoader`/zero-copy exists to avoid the cost of deserializing **large** accounts (thousands of bytes); every account in this program (`EventAccount`, `CheckpointAccount`, `ParticipantEventAccount`, `AttendanceAccount`) is well under 200 bytes (`#[derive(InitSpace)]`-sized structs of a handful of `Pubkey`/`u64`/`i64`/`u8` fields), so `Account<'info, T>`'s Borsh deserialization cost is negligible next to the CU figures above. A Pinocchio rewrite (this command's ~50-70% reduction claim) would only matter if CU cost were a binding constraint on transaction throughput or fee cost at scale — at under 20k CU against a 200k default (and Solana's per-block CU cap being the actual bottleneck for high-volume protocols, not this program's own budget), there is no scaling pressure here to justify trading Anchor's account-validation ergonomics and IDL generation for a manual zero-copy rewrite.

## Baseline

Saved to `.claude/benchmarks/cu-baseline.json` for future regression comparison — re-run the test above and diff against this file whenever an instruction handler changes.

## After Profiling

- [x] CU usage per instruction documented (table above, real measurements)
- [x] High-CU instructions identified — `check_in` (19.4k) and `create_checkpoint` (16.5k) are the highest, both still "Efficient" band
- [x] Baseline saved to `.claude/benchmarks/cu-baseline.json`
- [ ] Optimization plan for instructions > 150,000 CU — not applicable, nothing exceeds even 10% of that threshold
- [ ] Re-profile after optimizations — not applicable, no optimization needed at current usage levels
