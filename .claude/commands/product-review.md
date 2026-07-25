---
description: "Product quality review — first-time-user walkthrough, 8-dimension scorecard, prioritized fix roadmap. --harsh for the brutal roast variant"
---

<!-- Adapted from product-review + roast-my-product (sendaifun/solana-new), MIT © 2026 SendAI and Superteam. Telemetry removed. -->

You are reviewing a product the way its next user will experience it — not the way its builder hopes it works. Default mode is balanced and structured; `--harsh` is a stress test.

## Related Skills

- [ext/sendai/](../skills/ext/sendai/) — Solana Agent Kit patterns for agent-product reviews
- Playwright MCP (`mcp__playwright__*`) — drive the live app for the walkthrough below

## Modes

| Invocation | Tone | Output contract |
|------------|------|-----------------|
| `/product-review` | Balanced, constructive | 8-dimension scorecard + bucketed roadmap |
| `/product-review --harsh` | Brutal but constructive | 10 weighted dimensions + verdict + exactly 3 fixes |

This is not a code review — route code quality to `/diff-review` and security to `/audit-solana` / `/audit-infra`.

## Step 1: Gather Context

Ask before reviewing — never assume:

- What is the product? (URL, repo, or running build)
- Who is the target user, specifically?
- What is the ONE core action a user should complete?
- Stage: prototype / MVP / beta / launched?

If no product exists yet, stop and suggest `/plan-feature` or `/scaffold` instead.

## Step 2: First-Time-User Walkthrough

Become the target user from Step 1. If a URL or local build is available, **offer to drive it live with the playwright MCP** (`browser_navigate` → `browser_snapshot` → click through the core flow); otherwise walk the screens/code. Record an observation per checkpoint:

- [ ] Landing explains what this is in < 5 seconds, without crypto jargon
- [ ] Value is visible BEFORE wallet connect (no wallet-gate with nothing behind it)
- [ ] First meaningful action reachable in < 60 seconds, path obvious
- [ ] Transaction preview shows what will happen + cost before the wallet popup
- [ ] Pending → confirmed states visible, with explorer link
- [ ] A failed transaction explains why and what to try next
- [ ] After the first action, it's clear what to do next — and there's a reason to return

Document every friction point with where it happens and what the user sees.

## Step 3: Score the 8 Dimensions (default mode)

Score each 1–10. Every score requires evidence from Step 2 — "onboarding 6/10 because step 3 demands a wallet without explaining why" — never a bare number.

| # | Dimension | What you're judging |
|---|-----------|---------------------|
| 1 | Onboarding | Landing → first meaningful action: steps, friction, wallet deferral |
| 2 | Core-loop clarity | Is the repeated action obvious? Is there a reason to come back? |
| 3 | Empty states | Zero-balance, no-history, no-results screens guide instead of confuse |
| 4 | Error states | Tx failures human-readable, recovery guidance, no silent failures |
| 5 | Performance | Load time, tx feedback latency, informative loading states, data freshness |
| 6 | Trust signals | Audits, team, volume, social proof, comprehensible approval dialogs |
| 7 | Mobile | Responsive layout, touch targets, wallet deep-linking |
| 8 | Docs | Can a user self-serve answers? README/help/FAQ accuracy |

Overall = average, one decimal. For each dimension note: working well / needs improvement / one concrete fix for the biggest issue.

## Step 4: Build the Roadmap (default mode)

Bucket every fix, ordered by impact within each bucket:

- **Quick wins** (< 1 day) — copy changes, error-message rewrites, missing links
- **Medium** (1–3 days) — flow restructuring, state handling, mobile fixes
- **Major** (1 week+) — onboarding redesign, retention mechanics, new surfaces

Distinguish "nice to have" from "users are bouncing here" — the roadmap leads with the latter.

## Default Output

```
## Executive Summary
[2-3 sentences: overall quality, biggest strength, biggest risk]

## Scorecard
| Dimension | Score | Evidence |
|-----------|-------|----------|
| Onboarding | x/10 | ... |
| ... (all 8) | | |
| **Overall** | **x/10** | |

## Top 3 Strengths
1. [Strength] — [specific evidence]

## Top 3 Improvements
1. [Change] — [expected impact]

## Roadmap
### Quick wins (< 1 day)
- [ ] [Fix] — [impact]
### Medium (1–3 days)
- [ ] ...
### Major (1 week+)
- [ ] ...
```

---

## `--harsh` Mode

A stress test, not a review. Find every weakness before users and investors do.

### Tone rules

- Lead with the worst. No compliment sandwiches, no "overall it's pretty good".
- Every criticism = **what's wrong → why it matters → what good looks like**. Harshness without a fix is noise.
- Scores above 7 require justification — don't be generous.
- Call out crypto-for-crypto's-sake plainly: if Postgres replaces the chain and nothing breaks, say so.
- Never mean for its own sake — channel a YC partner with zero patience for hand-waving, not a heckler.

### The 10 weighted dimensions

| # | Dimension | Weight | Kill question |
|---|-----------|--------|---------------|
| 1 | Value proposition | **2x** | Explain it in one sentence without "decentralized/protocol/ecosystem" — can you? |
| 2 | Crypto necessity | 1x | What breaks if the chain becomes a database? Nothing → 1–3 |
| 3 | Target user clarity | 1x | Could you DM 10 real target users on Twitter right now? |
| 4 | First-time UX | 1x | Wallet-gate before any value shown? Jargon wall? |
| 5 | Core loop | 1x | What triggers a day-7 return? No answer → no loop |
| 6 | Moat | 1x | A funded competitor clones this in a weekend — what saves you? |
| 7 | Technical execution | 1x | What happens when the RPC dies mid-transaction? |
| 8 | Naming & messaging | 1x | Heard once — can you spell it and repeat the pitch? |
| 9 | Monetization | 1x | Token goes to zero — does the business survive? |
| 10 | Market timing | 1x | Why now? What changed in the last 6 months? |

Weighted total /110: 90+ exceptional · 70–89 strong · 50–69 needs work · 30–49 rethink · <30 start over.

### Harsh output — exactly this structure

```
## Verdict
[ONE devastating sentence — the single most damning truth]

## Scorecard
| Dimension | Score | Justification |
(all 10 rows + weighted total /110)

## The Worst Issues (top 3-5)
### 1. [Issue]
**What's wrong**: ... **Why it matters**: ... **What good looks like**: ...

## Fix These Now — exactly 3, no more, no less
1. **Highest impact**: [the fix that affects the most users]
2. **Easiest win**: [lowest effort, meaningful improvement]
3. **Existential**: [if this isn't fixed, the product dies]
```

## Checklist

- [ ] Context gathered before reviewing (product, user, core action, stage)
- [ ] Walkthrough done as the target user — live via playwright when possible
- [ ] Every score backed by specific evidence
- [ ] Default: 8 dimensions + bucketed roadmap | Harsh: 10 weighted + verdict + exactly 3 fixes
- [ ] Usability prioritized over aesthetics ("I dislike the design" ≠ "users can't complete the task")
- [ ] Code/security findings routed to `/diff-review`, `/audit-solana`, `/audit-infra` — not mixed in
