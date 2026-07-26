# EventQuest

An attendance and gamification platform for in-person events (workshops,
conferences, hackathons, booth visits) on **Solana Devnet**. Participants
connect a wallet, scan a rotating QR code at a checkpoint, and sign a
real on-chain check-in transaction — co-signed by the event's attestor —
that provably records their attendance and awards points.

Built as the challenge project for the Solana development workshop run by
**Superteam Brasil** at **TDC Floripa 2026**.

**Live app**: [eventquest.paxtel.com.br](https://eventquest.paxtel.com.br)

Full functional/technical spec: `EventQuest_Especificacao_IA_Solana_AI_Kit_EN_Dark.md`.

## Status

MVP complete on Solana Devnet. Every item in the spec's completion
checklist (§28) is satisfied with real evidence, not mocks or
local-validator-only claims — see [Devnet proof](#devnet-proof) below.

- **Program ID (Devnet)**: [`CuULC7T6d49mf89sRPU8WLeYfkF74kDyEKvfCe2rXq8H`](https://explorer.solana.com/address/CuULC7T6d49mf89sRPU8WLeYfkF74kDyEKvfCe2rXq8H?cluster=devnet)
- **Deploy record**: [`deployments/devnet.json`](deployments/devnet.json)
- **Full Devnet test report**: [`docs/DEVNET_TEST_REPORT.md`](docs/DEVNET_TEST_REPORT.md)
- **Security audits**: [`docs/security-audit-2026-07-26.md`](docs/security-audit-2026-07-26.md) (program), [`docs/audits/infra-2026-07-26.md`](docs/audits/infra-2026-07-26.md) (infrastructure), deferred findings in [`docs/SECURITY.md`](docs/SECURITY.md)

Mainnet is explicitly out of scope. `scripts/verify-devnet-config.sh`
independently fails (non-zero exit) on any Mainnet signal in the
configured RPC/network.

## Architecture

```
programs/eventquest/       Anchor program — Event, Checkpoint,
                            ParticipantEvent, Attendance PDAs
crates/eventquest-chain/    Shared Rust account/instruction layer
                            (generated from the built IDL)
crates/eventquest-domain/   Shared domain types (error codes, ids, enums)
crates/eventquest-config/   Env-based config loading
apps/api/                   Axum backend — auth, events, checkpoints,
                            QR rotation, check-in state machine
apps/indexer/                Separate worker: decodes on-chain
                            AttendanceRecorded events, the source of
                            truth for "attendance confirmed"
apps/web/                   Next.js 16 / React 19 / @solana/kit frontend
packages/chain-client/      Codama-generated TS client (mirrors
                            crates/eventquest-chain)
```

Why a hybrid on-chain/off-chain split, why the frontend never trusts its
own transaction-submission callback, why the indexer (not the API) is the
final source of truth for a confirmed check-in — see
[`docs/adr/ADR-001-architecture.md`](docs/adr/ADR-001-architecture.md) and
[`docs/adr/ADR-002-solana-client.md`](docs/adr/ADR-002-solana-client.md).

## On-chain program

| Instruction | Who signs | What it does |
|---|---|---|
| `initialize_event` | organizer | Creates the `EventAccount` PDA |
| `update_event_status` | organizer | Draft → Active → Paused/Finished/Cancelled |
| `create_checkpoint` | organizer | Creates a `CheckpointAccount` with its own attestor, window, and points |
| `update_checkpoint` | organizer | Adjusts schedule/points/attestor (points locked once check-ins exist) |
| `join_event` | participant | Creates the participant's `ParticipantEventAccount` (once, before any check-in) |
| `check_in` | participant **+** attestor | Creates the `AttendanceAccount` PDA — fails outright on a repeat attempt, since the PDA already exists |
| `finish_participant_event` | organizer | Marks a participant's event progress complete |

Points always come from `CheckpointAccount`, never from a client-supplied
argument. See `docs/security-audit-2026-07-26.md` for the full account/
arithmetic/CPI security review.

## Prerequisites

- Node.js ≥ 20, pnpm 9.15.0 (`packageManager` pinned in `package.json`)
- Rust (stable) + `anchor-cli` 1.1.2 + `avm` 1.1.2
- Docker (for local Postgres/Redis)
- [Surfpool](https://github.com/txtx/surfpool) CLI (local Devnet-equivalent validator, used by `scripts/e2e-local.sh`)
- A Devnet-funded keypair for `ATTESTOR_KEYPAIR_PATH` if you want to run the Devnet-facing scripts yourself

## Setup

```bash
pnpm install
cp .env.example .env
# Fill in at least: SOLANA_PROGRAM_ID, ATTESTOR_KEYPAIR_PATH,
# QR_SIGNING_SECRET, AUTH_SIGNING_SECRET (see .env.example for every field)

docker compose up -d postgres redis
```

## Quick start (local development)

```bash
# Backend
cargo run -p eventquest-api        # :3001, refuses to start off Devnet
cargo run -p eventquest-indexer    # background worker

# Frontend
pnpm --filter=./apps/web dev       # :3000, proxies /api/v1/* to apps/api
```

Open `http://localhost:3000`, connect a Devnet-funded wallet (Phantom/
Solflare/Backpack), and create an event from `/admin`.

## Testing

```bash
# Rust: program (Mollusk) + backend + indexer + shared crates
cargo fmt --check
cargo clippy --workspace --all-targets -- -W clippy::all -D warnings
cargo test --workspace              # 42 tests, 4 additional live-Devnet
                                     # tests gated behind --ignored

# TypeScript: frontend + generated client
pnpm -r --filter=./apps/web --filter=./packages/chain-client typecheck
pnpm --filter=./apps/web lint
pnpm test:ts                        # 318 Vitest assertions + 4 client tests
pnpm --filter=./apps/web build

# Local E2E — real Surfpool validator, real Playwright browser, real
# WebCrypto-signed wallet, full vertical scenario
pnpm --filter=./apps/web exec playwright install chromium  # once
./scripts/e2e-local.sh

# Devnet-facing (each costs a small amount of real Devnet SOL from the
# attestor keypair — see docs/DEVNET_TEST_REPORT.md's "known limitations")
./scripts/verify-devnet-config.sh
pnpm devnet:smoke
./scripts/devnet-e2e.sh
```

CI (`.github/workflows/ci.yml`) runs fmt/clippy/test and typecheck/lint/
test/build on every push and PR to `main`. The Devnet-facing scripts are
deliberately not in CI — they need a funded keypair and take minutes.

## Devnet proof

Every claim below is a real, independently-verifiable transaction —
never a mock, a local-validator-only result, or a simulated transaction
(spec §0.8's explicit bar):

- Program deployed and executable at `CuULC7T6d49mf89sRPU8WLeYfkF74kDyEKvfCe2rXq8H` (`deployments/devnet.json`)
- `pnpm devnet:smoke` (`scripts/devnet-smoke-test.ts`): real event → checkpoint → join → check-in → Attendance PDA read-back and validated → duplicate check-in attempted and rejected on-chain, exits non-zero on any failed assertion
- `./scripts/devnet-e2e.sh`: the same flow driven through the real production frontend, real `apps/api`, real Postgres/Redis, and the real indexer against real Devnet — passed, independently re-verified afterward by querying Postgres directly and confirming the recorded signature `finalized` via RPC
- Full evidence (signatures, PDA addresses, Explorer links, wallet pubkeys, versions, known limitations): [`docs/DEVNET_TEST_REPORT.md`](docs/DEVNET_TEST_REPORT.md)

## Security

- On-chain program audit: [`docs/security-audit-2026-07-26.md`](docs/security-audit-2026-07-26.md) — 0 critical, 0 high, 0 medium findings
- Infrastructure audit: [`docs/audits/infra-2026-07-26.md`](docs/audits/infra-2026-07-26.md) — 2 medium findings, both fixed in the same pass (log-level wiring, sensitive tokens in query-string logs)
- All deferred (non-blocking) findings, with severity/evidence/impact/mitigation/justification: [`docs/SECURITY.md`](docs/SECURITY.md)
- Secrets: never committed (verified against full git history, not just the working tree) — see `.gitignore` and the infra audit's Phase 1

## Deployment

Live on [Railway](https://railway.app): `apps/web`, `apps/api`, and
`apps/indexer` each run from their own Dockerfile
(`infrastructure/docker/{web,api,indexer}.Dockerfile`), with managed
Postgres and Redis add-ons. Config-as-code per service:
`railway.web.json`, `railway.api.json`, `railway.indexer.json`.

Two things worth knowing if you deploy this yourself:
- `apps/web`'s rewrite destination (`API_URL`) and every `NEXT_PUBLIC_*`
  var are resolved once, at `next build` time, and baked into the
  standalone output — they're **not** re-read at container start, so the
  Dockerfile declares them as `ARG`s (Docker never forwards a platform's
  service variables into a build unless a matching `ARG` exists) and any
  change to them needs a real rebuild, not just a restart.
- `apps/api` binds to `$PORT` if the platform sets one (Railway does),
  falling back to the port embedded in `API_URL` for local dev/
  docker-compose, which never set `$PORT` at all.

## Known limitations

See `docs/DEVNET_TEST_REPORT.md`'s "Known limitations" section and
`docs/SECURITY.md` SEC-07 for the full list. Highlights:
cNFT/Bubblegum passports, ranking, marketplace, token, payments,
geofencing, and a native mobile app are explicitly out of MVP scope per
spec §4/§30.

## Roadmap

- **Sponsored check-in fee**: today, both the organizer (event/checkpoint
  creation) and the participant (`join_event`, `check_in`) pay their own
  transaction fee and account rent — free on Devnet via faucet, but real
  cost if this ever moved beyond Devnet. A **fee relayer** would let the
  event owner (or the app itself) cover the participant's check-in cost:
  the participant still signs to prove it's really their wallet, but a
  separate funded backend keypair is the transaction's fee payer instead.
  Meaningful for a non-crypto-native audience who shouldn't need SOL in
  their wallet just to check in. Not implemented yet — see this repo's
  own discussion of the tradeoffs (backend now needs a funded, actively
  managed relayer wallet, and rate limiting matters more once check-ins
  are free for participants to attempt).
- **Passport visible in the wallet itself**: today, "My passport"
  (`/participant/passport`) only exists as a page on this site — it reads
  the participant's on-chain accounts and Postgres, but nothing is
  actually minted, so a participant's own wallet app (Phantom, Solflare,
  etc.) has nothing to show natively. Minting an NFT — likely a
  compressed NFT via **Bubblegum** for cost reasons — per attendance or
  per completed event would make it show up directly in the wallet's own
  Collectibles view, no visit to the app required. Explicitly out of MVP
  scope per spec §4/§30; not implemented.

## Operations

See [`docs/RUNBOOK.md`](docs/RUNBOOK.md) for local troubleshooting, how to
reset a stuck indexer cursor, and how to rotate the attestor keypair.
