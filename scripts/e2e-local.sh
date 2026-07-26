#!/usr/bin/env bash
# Phase 5 local E2E harness (spec §25.4): stands up a real, local stack —
# Surfpool (a real SVM, not a mock), Postgres/Redis, apps/api,
# apps/indexer, and a production apps/web build — deploys the real
# eventquest program to Surfpool, then runs the Playwright vertical
# scenario against all of it. Everything here is real: real transactions,
# real signatures (via a genuine WebCrypto Ed25519 keypair — see
# apps/web/tests/e2e/inject-test-wallet.js), real program execution. Only
# the cluster is local instead of public Devnet — that's what makes this
# Phase 5 (fast, repeatable, no faucet/rate-limit dependency) rather than
# Phase 6 (the real Devnet proof).
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
E2E_DIR="$ROOT_DIR/.e2e-local"
mkdir -p "$E2E_DIR"

SURFNET_RPC_PORT=8899
SURFNET_WS_PORT=8900
API_PORT=3101
WEB_PORT=3100

SURFPOOL_PID=""
API_PID=""
INDEXER_PID=""
WEB_PID=""

log() { echo "[e2e-local] $*"; }

cleanup() {
  log "tearing down..."
  [[ -n "$WEB_PID" ]] && kill "$WEB_PID" 2>/dev/null || true
  [[ -n "$INDEXER_PID" ]] && kill "$INDEXER_PID" 2>/dev/null || true
  [[ -n "$API_PID" ]] && kill "$API_PID" 2>/dev/null || true
  [[ -n "$SURFPOOL_PID" ]] && kill "$SURFPOOL_PID" 2>/dev/null || true
  wait 2>/dev/null || true
}
trap cleanup EXIT

wait_for_http() {
  local url="$1" name="$2" attempts=0
  until curl -s -o /dev/null "$url"; do
    attempts=$((attempts + 1))
    if [[ $attempts -gt 60 ]]; then
      log "ERROR: $name did not become ready at $url"
      exit 1
    fi
    sleep 1
  done
  log "$name is ready ($url)"
}

log "resetting Postgres/Redis (fresh state each run)"
# Postgres data persists in a docker volume across runs, but Surfpool is a
# brand-new ephemeral chain every time (`--offline`, starting near slot 0).
# A stale `chain_sync_cursors` row from a previous run's much-higher slot
# number would make apps/indexer think it's already caught up forever,
# since the fresh chain never reaches that slot — so every run starts from
# a clean volume rather than risking that mismatch.
(cd "$ROOT_DIR" && docker compose down -v postgres redis 2>/dev/null || true)
(cd "$ROOT_DIR" && docker compose up -d postgres redis)

log "waiting for a fresh Postgres to accept connections"
until (cd "$ROOT_DIR" && docker compose exec -T postgres pg_isready -U eventquest >/dev/null 2>&1); do
  sleep 1
done

log "generating ephemeral deployer/attestor keypairs"
solana-keygen new --no-bip39-passphrase --force -o "$E2E_DIR/deployer.json" >/dev/null
solana-keygen new --no-bip39-passphrase --force -o "$E2E_DIR/attestor.json" >/dev/null
DEPLOYER_PUBKEY="$(solana-keygen pubkey "$E2E_DIR/deployer.json")"
ATTESTOR_PUBKEY="$(solana-keygen pubkey "$E2E_DIR/attestor.json")"
log "deployer: $DEPLOYER_PUBKEY"
log "attestor: $ATTESTOR_PUBKEY"

log "building the on-chain program (anchor build)"
(cd "$ROOT_DIR" && anchor build)

log "starting Surfpool (offline local Surfnet)"
surfpool start --offline --no-deploy --ci \
  --port "$SURFNET_RPC_PORT" --ws-port "$SURFNET_WS_PORT" \
  --airdrop "$DEPLOYER_PUBKEY" --airdrop "$ATTESTOR_PUBKEY" --airdrop-amount 50000000000 \
  >"$E2E_DIR/surfpool.log" 2>&1 &
SURFPOOL_PID=$!
wait_for_http "http://127.0.0.1:$SURFNET_RPC_PORT/health" "Surfpool RPC"

log "deploying eventquest program to the local Surfnet"
solana program deploy "$ROOT_DIR/target/deploy/eventquest.so" \
  --program-id "$ROOT_DIR/target/deploy/eventquest-keypair.json" \
  --keypair "$E2E_DIR/deployer.json" \
  --url "http://127.0.0.1:$SURFNET_RPC_PORT"

PROGRAM_ID="$(solana-keygen pubkey "$ROOT_DIR/target/deploy/eventquest-keypair.json")"

# Shared env for apps/api and apps/indexer. SOLANA_NETWORK is the literal
# string the config gate requires — this is a *local* Surfnet standing in
# for the real cluster, which is exactly what Phase 5 (vs. Phase 6) means.
export NODE_ENV=test
export PUBLIC_APP_URL="http://localhost:$WEB_PORT"
export API_URL="http://localhost:$API_PORT"
export DATABASE_URL="postgresql://eventquest:eventquest@localhost:5432/eventquest"
export REDIS_URL="redis://localhost:6379"
export SOLANA_NETWORK=devnet
export SOLANA_RPC_HTTP_URL="http://127.0.0.1:$SURFNET_RPC_PORT"
export SOLANA_RPC_WS_URL="ws://127.0.0.1:$SURFNET_WS_PORT"
export SOLANA_PROGRAM_ID="$PROGRAM_ID"
export ATTESTOR_KEYPAIR_PATH="$E2E_DIR/attestor.json"
export QR_SIGNING_SECRET="e2e-local-qr-secret"
export AUTH_SIGNING_SECRET="e2e-local-auth-secret"
export QR_ROTATION_SECONDS=15
export QR_GRACE_SECONDS=5
export CHECKIN_GRANT_SECONDS=30
export SESSION_COOKIE_NAME=eventquest_session
export SESSION_TTL_SECONDS=86400
export LOG_LEVEL=info
export NEXT_PUBLIC_DEFAULT_LOCALE=en-US
export NEXT_PUBLIC_THEME=dark
export NEXT_PUBLIC_ACCENT_COLOR=orange
export NEXT_PUBLIC_SOLANA_NETWORK=devnet
export NEXT_PUBLIC_SOLANA_PROGRAM_ID="$PROGRAM_ID"

log "starting apps/api on :$API_PORT"
(cd "$ROOT_DIR" && cargo run -p eventquest-api >"$E2E_DIR/api.log" 2>&1) &
API_PID=$!
wait_for_http "http://localhost:$API_PORT/health/ready" "apps/api"

log "starting apps/indexer"
(cd "$ROOT_DIR" && cargo run -p eventquest-indexer >"$E2E_DIR/indexer.log" 2>&1) &
INDEXER_PID=$!

log "building apps/web"
(cd "$ROOT_DIR/apps/web" && pnpm build >"$E2E_DIR/web-build.log" 2>&1)

log "starting apps/web on :$WEB_PORT"
(cd "$ROOT_DIR/apps/web" && PORT="$WEB_PORT" pnpm start >"$E2E_DIR/web.log" 2>&1) &
WEB_PID=$!
wait_for_http "http://localhost:$WEB_PORT/" "apps/web"

log "running Playwright vertical scenario"
set +e
(cd "$ROOT_DIR/apps/web" && E2E_BASE_URL="http://localhost:$WEB_PORT" E2E_RPC_URL="http://127.0.0.1:$SURFNET_RPC_PORT" pnpm exec playwright test)
EXIT_CODE=$?
set -e

log "done (exit code $EXIT_CODE)"
exit $EXIT_CODE
