# Product Review — EventQuest — 2026-07-26

**Method**: code/flow walkthrough (component code, `en-US.ts` copy, and the
real E2E test assertions in `apps/web/tests/e2e*/`) — no live build was
running at review time, so this is not a fresh-eyes click-through with the
Playwright MCP; treat mobile-viewport and real-latency-feel observations
below as lower-confidence than the copy/flow-structure ones. Target user:
an event participant using a personal phone to check into a checkpoint
they've just walked up to; secondary user: the organizer running the
event from a laptop.

## Executive Summary

The core loop (scan → sign → confirm) is short, explains itself at every
step, and never lies about state — every intermediate status has its own
distinct copy (`validatingQr` → `preparingTransaction` →
`awaitingSignature` → `submitting` → `confirming` →
`transactionConfirmed`), and failure states are equally specific rather
than a generic "something went wrong." The biggest risk isn't a UX flaw so
much as unavoidable Solana-Devnet reality: check-in confirmation latency
is whatever public Devnet RPC gives you that moment, and the product has
no fallback messaging for "this is taking longer than usual."

## Scorecard

| Dimension | Score | Evidence |
|---|---|---|
| Onboarding | 7/10 | `/events` and `/events/[slug]` are wallet-gate-free (public browsing works before connecting) — value is visible before commitment. Connecting is one click, but sign-in immediately follows with no explanation screen of *why* a signature is needed beyond `signingIn: "Waiting for wallet signature"` — a first-time crypto-unfamiliar user sees a wallet popup with no product-authored context first. |
| Core-loop clarity | 8/10 | The check-in state machine (`apps/web/src/app/check-in/page.tsx`) has one job and one obvious next action at every step; `RotatingQrCode` + `newCodeIn ... seconds` gives a clear reason to act promptly. No "why check in again" reason to return is surfaced (points/leaderboard aren't shown until `/participant/passport`, which isn't linked from the check-in result screen). |
| Empty states | 8/10 | `passport.empty: "You haven't checked in to any events yet."` + a `browseEvents` CTA, `participants.empty`, both guide rather than dead-end. |
| Error states | 8/10 | `qrExpired`, `alreadyCheckedIn`, `transactionRejected` are each distinct, human, and actionable (`tryAgain`/`scanAnother`) — verified these are real, reachable states via `apps/web/tests/e2e/vertical-scenario.spec.ts`'s duplicate-check-in assertion (`getByRole("alert")`), not just copy that's never wired up. No copy exists for "RPC/API unreachable" specifically — it likely falls through to a generic error, unverified without a live run. |
| Performance | 6/10 | `confirming: "Confirming on Solana Devnet"` is shown while waiting, which is honest, but there's no elapsed-time indicator or "this can take up to N seconds on Devnet" framing — a real participant standing at a booth with a 10-30s public-RPC confirmation delay and only a static string has no signal whether it's still working or stuck. This is the single highest-impact gap found. |
| Trust signals | 7/10 | Every on-chain action links to Explorer (`viewTransaction`), and the `DevnetBanner` component keeps network context always visible (`networkMismatchWarning` catches the wrong-network case explicitly) — strong technical trust signals. No social proof (attendee counts, event branding) since that's out of MVP scope. |
| Mobile | 6/10 | `useQrScanner` prefers the rear camera and the layout is mobile-first per the theme spec, and this was exercised through a real phone-shaped viewport only indirectly (Playwright's default desktop viewport in the E2E suite, not a mobile emulation profile) — genuine mobile touch-target/viewport testing hasn't been done live. |
| Docs | 7/10 | `README.md` + `docs/RUNBOOK.md` are thorough for developers/operators; there's no in-product help/FAQ for participants (e.g. "what is a wallet" for a first-time crypto user at a conference), which is reasonable for an MVP but worth flagging. |
| **Overall** | **7.1/10** | |

## Top 3 Strengths

1. **Honest, granular status copy** — every step of check-in has its own string, verified wired to real state transitions by the E2E test's assertions, not just written and forgotten.
2. **Public browsing before any wallet gate** — `/events` requires no connection, which is the correct call per spec and avoids the single most common Web3-onboarding failure (wallet-gate before any value shown).
3. **Devnet/network context is never ambiguous** — `DevnetBanner`, per-action Explorer links, and an explicit wrong-network warning mean a user is never confused about what network they're on or whether an action actually happened on-chain.

## Top 3 Improvements

1. **Add elapsed-time / "still working" framing to the `confirming` state** — the biggest real-world friction point given Devnet's variable confirmation latency; even a simple "this usually takes 5-15 seconds" or a spinner-with-timer would prevent a participant from assuming the app is frozen.
2. **Surface a reason to return after check-in** — link points/passport progress directly from `/check-in/result`, not just leave it reachable via nav.
3. **A one-line "why am I signing this" micro-explainer** before the first wallet popup a new user sees, distinct from the generic `signingIn` waiting state.

## Roadmap

### Quick wins (< 1 day)
- [ ] Link `/participant/passport` from `/check-in/result`'s confirmed state — copy change + one link, no new logic.
- [ ] Add a "this can take up to 30 seconds on Devnet" line to the `confirming` copy.

### Medium (1–3 days)
- [ ] Elapsed-time indicator during `confirming` (timer or animated progress, not just static text).
- [ ] Verify and, if needed, fix mobile touch-target sizing with a real device/emulated-viewport pass (not yet done live).
- [ ] Add copy + handling for an unreachable-API/RPC error state distinct from a rejected/expired QR, if not already covered.

### Major (1 week+)
- [ ] First-time-user micro-onboarding (what is a wallet / why sign this) — likely only worth it if this product is used at events with a meaningfully non-crypto-native audience, which is a product decision, not just a UX one.
