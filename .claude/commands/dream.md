---
description: "Memory consolidation — dedupe, contradiction-check, prune, and re-rank MEMORY.md + CLAUDE.md Project Learnings. Run after major refactors"
---

<!-- Schema adapted from the learn skill (gstack via sendaifun/solana-new), MIT © 2026 SendAI and Superteam. Local-only — no sync, no telemetry. -->

You are consolidating this project's memory. Like sleep, this pass doesn't add new knowledge — it merges duplicates, resolves contradictions, discards what's stale, and strengthens what's proven. Touch ONLY `MEMORY.md` and (with confirmation) the `## Project Learnings` section of `CLAUDE.md`. Never modify project code.

## Learning Schema

Every entry in `MEMORY.md` is typed and structured. Sections: `## Patterns`, `## Pitfalls`, `## Preferences`, `## Architecture`, `## Tools`.

```markdown
### kebab-case-key
- **Insight:** One clear sentence
- **Confidence:** N/10
- **Source:** command/agent/session that produced it (e.g. debug-user-tx, manual)
- **Files:** relative/path.rs, other/path.ts   (optional)
- **Date:** YYYY-MM-DD
```

Entries missing fields get them backfilled during consolidation (unknown source → `legacy`, missing date → today, missing confidence → 5/10).

## Pipeline

### 1. Collect

- Read `MEMORY.md`. If missing, create it with the five section headers above and report "fresh memory initialized" — then skip to step 6.
- Read the `## Project Learnings` section of `CLAUDE.md` if present (subsections like Recurring Issues / Fix Patterns / Config Conventions). These are the already-exported, always-loaded learnings — treat them as the authoritative set when checking contradictions.
- Inventory: total entries, entries per section, malformed entries.

### 2. Dedupe / Merge

- Same key appearing more than once → keep the latest dated entry, fold any extra detail from older ones into its Insight, take the max confidence. Older duplicates are deleted (consolidation is the one place append-only history gets compacted).
- Different keys, same meaning (read the insights, don't just string-match) → merge under the clearer key, note the alias in the Insight if useful.

### 3. Contradiction Check

- Same topic, opposing advice ("use X" vs "avoid X") — within MEMORY.md, or between MEMORY.md and CLAUDE.md Project Learnings.
- Resolution order: (a) higher confidence + newer date wins; (b) if the referenced files reveal which is currently true, verify against the code; (c) genuinely ambiguous → ask the user, presenting both entries.
- The losing entry is deleted, and the winner's Insight gains a one-line "supersedes: <old-key>" note.

### 4. Prune

Flag for removal:

- **Dead references** — any entry whose Files no longer exist (check with Glob; if some files survive, just trim the Files list)
- **Stale** — confidence ≤ 4 AND older than 90 days with no re-confirmation
- **Transient leftovers** — entries describing in-progress work, old branch state, or one-off incidents that can't recur
- **Redundant** — restates something already in README/CLAUDE.md verbatim

Auto-remove dead references and transient leftovers. For stale-but-plausible entries, ask the user: remove / keep (re-date to today) / update insight.

### 5. Re-rank

Within each section, order entries by confidence descending, then date descending. Highest-signal knowledge reads first.

### 6. Write Back (cap enforcement)

`MEMORY.md` loads every session start — hard cap **200 lines / 25KB**, index-pointer style (one-line insights pointing at files, not essays). The richer typed fields (confidence, dates, insight) used during steps 2–5 are processing-only — the written `MEMORY.md` keeps the index-pointer format, with that metadata living in the linked detail files, not the index.

- If over cap after steps 2–5: compress Insights to one line, drop optional Files where obvious, then remove lowest-confidence entries until under cap (tell the user what was cut).
- Write the consolidated file. Show a before/after diff summary, not the full file.

### 7. Export to CLAUDE.md (optional, confirm first)

An entry graduates from MEMORY.md to CLAUDE.md `## Project Learnings` when ALL hold:

- Confidence ≥ 8/10
- Survived at least one prior consolidation (≥ 30 days old or re-confirmed)
- General — applies to the whole project, not one file's quirk
- Short — exports as a single bullet: `- **key:** insight (N/10)`

Propose the export list; on confirmation append bullets under the matching subsection and remove the originals from MEMORY.md (CLAUDE.md is always loaded — keeping both wastes tokens).

## Write / Don't-Write Rules

| Write | Never write |
|-------|-------------|
| Reusable patterns, pitfalls that cost > 10 min, user preferences, architecture decisions with trade-offs, tool choices with reasons | Secrets, keys, tokens, wallet addresses tied to the user, or anything resembling credential material |
| One-line insights + file pointers | File contents, code blocks > 3 lines, command transcripts |
| Confirmed, dated knowledge | Transient state: current branch, in-flight tasks, TODO lists, session-specific context |
| | Anything already stated in README or CLAUDE.md |

If a candidate entry contains something from the "never" column, strip it or drop the entry — there is no exception path.

## Output

```
## Dream Report — <date>

| Stage | Result |
|-------|--------|
| Collected | N entries (P pat / Q pit / R pref / S arch / T tool), M malformed fixed |
| Merged | N duplicates folded |
| Contradictions | N resolved (list keys), N escalated to user |
| Pruned | N removed (dead refs: n, stale: n, transient: n) |
| Exported | N promoted to CLAUDE.md (or "none proposed") |
| Size | X lines / Y KB (cap: 200 / 25KB) ✓ |
```

## Checklist

- [ ] MEMORY.md read (or initialized) and CLAUDE.md Project Learnings collected
- [ ] Duplicates merged — latest wins, max confidence kept
- [ ] Contradictions resolved or escalated, never left side by side
- [ ] Every Files reference existence-checked before keeping
- [ ] No secrets / transient state / CLAUDE.md duplicates written
- [ ] Final MEMORY.md ≤ 200 lines and ≤ 25KB
- [ ] Exports confirmed with the user before touching CLAUDE.md
- [ ] Dream report printed — no other files modified
