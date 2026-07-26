#!/usr/bin/env bash
# Spec §25.6: the browser E2E scenario against real Devnet — real frontend,
# real apps/api, real Postgres/Redis, the real deployed eventquest program,
# and a real indexer. Unlike scripts/e2e-local.sh (Phase 5), there is no
# Surfpool/anchor build/deploy step here: the program is already live on
# Devnet (see deployments/devnet.json), so this script only stands up the
# off-chain services and points them at it.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
E2E_DIR="$ROOT_DIR/.e2e-devnet"
mkdir -p "$E2E_DIR"

API_PORT=3101
WEB_PORT=3100

API_PID=""
INDEXER_PID=""
WEB_PID=""

log() { echo "[devnet-e2e] $*"; }

cleanup() {
  log "tearing down..."
  [[ -n "$WEB_PID" ]] && kill "$WEB_PID" 2>/dev/null || true
  [[ -n "$INDEXER_PID" ]] && kill "$INDEXER_PID" 2>/dev/null || true
  [[ -n "$API_PID" ]] && kill "$API_PID" 2>/dev/null || true
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

log "verifying Devnet configuration (refuses to proceed on any Mainnet signal)"
bash "$ROOT_DIR/scripts/verify-devnet-config.sh"

log "ensuring Postgres/Redis are up"
(cd "$ROOT_DIR" && docker compose up -d postgres redis)

log "waiting for Postgres to accept connections"
until (cd "$ROOT_DIR" && docker compose exec -T postgres pg_isready -U eventquest >/dev/null 2>&1); do
  sleep 1
done

# Load the shared .env (SOLANA_RPC_HTTP_URL, SOLANA_PROGRAM_ID,
# ATTESTOR_KEYPAIR_PATH, etc. all already point at Devnet per
# scripts/verify-devnet-config.sh's check above) and override only what's
# specific to this local off-chain stack.
set -a
# shellcheck disable=SC1091
source "$ROOT_DIR/.env"
set +a

export NODE_ENV=production
export PUBLIC_APP_URL="http://localhost:$WEB_PORT"
export API_URL="http://localhost:$API_PORT"
export DATABASE_URL="postgresql://eventquest:eventquest@localhost:5432/eventquest"
export REDIS_URL="redis://localhost:6379"
export SESSION_COOKIE_NAME=eventquest_session
export SESSION_TTL_SECONDS=86400
export LOG_LEVEL=info
export NEXT_PUBLIC_DEFAULT_LOCALE=en-US
export NEXT_PUBLIC_THEME=dark
export NEXT_PUBLIC_ACCENT_COLOR=orange

log "starting apps/api on :$API_PORT (Devnet RPC: $SOLANA_RPC_HTTP_URL)"
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

log "running Playwright Devnet vertical scenario"
set +e
(cd "$ROOT_DIR/apps/web" && \
  E2E_BASE_URL="http://localhost:$WEB_PORT" \
  E2E_RPC_URL="$SOLANA_RPC_HTTP_URL" \
  E2E_RPC_WS_URL="$SOLANA_RPC_WS_URL" \
  ATTESTOR_KEYPAIR_PATH="$ATTESTOR_KEYPAIR_PATH" \
  pnpm exec playwright test --config=playwright.devnet.config.ts)
EXIT_CODE=$?
set -e

log "done (exit code $EXIT_CODE)"
exit $EXIT_CODE
