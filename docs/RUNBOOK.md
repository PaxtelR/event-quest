# Operations Runbook

Practical troubleshooting for running EventQuest — written from real
issues hit and fixed during this project's own Phase 5/6 E2E work, not
speculative.

## Starting the stack locally

```bash
docker compose up -d postgres redis
cargo run -p eventquest-api        # :3001 — refuses to start unless
                                    # SOLANA_NETWORK=devnet and
                                    # Postgres/Redis are reachable
cargo run -p eventquest-indexer
pnpm --filter=./apps/web dev       # :3000
```

Health checks: `GET http://localhost:3001/health/live` (process up) and
`/health/ready` (DB + Redis reachable).

## The indexer's cursor can go stale after a chain reset

**Symptom**: check-ins submit successfully (`apps/api` returns a real
transaction signature), but the participant never sees "Attendance
confirmed," the organizer's dashboard never shows updated points, and
`apps/indexer`'s logs are silent (no "indexed new transactions" lines,
ever).

**Root cause**: `apps/indexer` persists a cursor
(`chain_sync_cursors.last_processed_slot`) in Postgres, keyed by
`(network, program_id)`. Postgres survives restarts of the *chain*, but a
local Surfpool validator does not — every fresh `surfpool start` begins
at a low slot number. If Postgres has a cursor from a previous session at
a much higher slot, the indexer believes it's already caught up to a
future the current chain hasn't reached yet, and polls forever without
finding anything new.

**This exact bug was hit and fixed during this project's own Phase 5
work** — `scripts/e2e-local.sh` now runs `docker compose down -v postgres
redis` before every run specifically to prevent it. If you're running
services manually (not through the script) and hit the symptom above:

```bash
docker exec <postgres-container> psql -U eventquest -d eventquest \
  -c "delete from chain_sync_cursors;"
# restart apps/indexer — it will re-derive a correct cursor from the
# current chain's actual slot on its next tick
```

This is **not** a concern against real Devnet (slots only ever increase),
only against a local ephemeral validator.

## QR display shows "Check-in temporarily unavailable" despite an active checkpoint

**Symptom**: the checkpoint display page (`/admin/checkpoints/:id/display`)
shows the SSE connection as "Connected" but never renders a QR code, even
though the checkpoint is genuinely `active` in the database and the
`current-qr` REST endpoint returns a valid token when called directly.

**Root cause**: `next start`'s default response compression buffers the
SSE stream when the client sends `Accept-Encoding: gzip` (every real
browser does; `curl` without `--compressed` doesn't, which is why this is
easy to miss when debugging with `curl`). A deliberately long-lived SSE
connection under gzip compression may never flush a chunk to the client.
Already fixed — `apps/web/next.config.ts` sets `compress: false` — but if
you ever see this symptom again after touching that file, check `compress`
first before suspecting the backend.

## Rotating the attestor keypair

The attestor keypair (`ATTESTOR_KEYPAIR_PATH`, e.g.
`.secrets/attestor-devnet.json`) co-signs every check-in and is loaded
once at `apps/api` startup.

1. Generate a new keypair: `solana-keygen new -o .secrets/attestor-devnet-new.json`
2. Fund it on Devnet (small amount — it never pays transaction fees itself,
   only needs to exist as a signer): `solana airdrop 1 <new-pubkey> --url devnet`
3. For **new** checkpoints: `apps/api` will use the new keypair automatically
   once `ATTESTOR_KEYPAIR_PATH` is updated and the process restarted.
4. For **existing active** checkpoints created with the old attestor: they
   must be updated via `update_checkpoint` (organizer-signed) with the new
   attestor pubkey, or check-ins against them will fail `InvalidAttestor`
   until updated — see `programs/eventquest/src/instructions/check_in.rs`.
5. Swap the file, restart `apps/api`.

Never commit the keypair file — `.secrets/` and `*attestor*.json` are
gitignored; verify with `git check-ignore .secrets/attestor-devnet.json`
before assuming it's safe.

## Devnet-facing scripts spend real (small) SOL

`scripts/devnet-smoke-test.ts` and `scripts/devnet-e2e.sh` fund fresh
throwaway keypairs via a direct transfer from the attestor keypair (not
the public faucet, which is rate-limited enough to break repeat automated
runs) — roughly 0.03 SOL per run. Check the attestor's balance before a
run if you haven't topped it up in a while:

```bash
solana balance <attestor-pubkey> --url devnet
solana airdrop 2 <attestor-pubkey> --url devnet   # if low
```

## Resetting the local E2E stack entirely

```bash
./scripts/e2e-local.sh
```

Tears down and recreates the Postgres/Redis volumes, generates fresh
ephemeral deployer/attestor keypairs, builds and deploys the program to a
fresh local Surfpool validator, and runs the full Playwright vertical
scenario. Logs land in `.e2e-local/*.log` (gitignored).

## Common `verify-devnet-config.sh` failures

- `SOLANA_NETWORK is '', must be exactly 'devnet'` — `.env` missing or not
  loaded; check you're running from the repo root.
- `RPC genesis hash ... does not match known Devnet genesis hash` — your
  `SOLANA_RPC_HTTP_URL` points somewhere other than Devnet (or is
  unreachable); the check queries the RPC directly rather than trusting
  the URL string, so a typo'd or wrong endpoint is still caught.
- `Program ... is not executable on Devnet` — the program ID in `.env`
  doesn't match anything deployed, or the RPC node hasn't caught up;
  cross-check against `deployments/devnet.json`.
