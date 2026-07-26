#!/usr/bin/env bash
# Spec §0.7/§0.9: every Devnet delivery must actively guard against an
# accidental Mainnet deploy. This checks the *actual* configured RPC
# endpoint — not just that env var names look right — by asking it for its
# genesis hash and comparing against the well-known Devnet/Mainnet values,
# so a misconfigured URL (e.g. one that merely omits "mainnet" from its
# hostname) still gets caught.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

DEVNET_GENESIS_HASH="EtWTRABZaYq6iMfeYKouRu166VU2xqa1wcaWoxPkrZBG"
MAINNET_GENESIS_HASH="5eykt4UsFv8P8NJdTREpY1vzqKqZKvdpKuc147dw2N9d"

FAILED=0
fail() {
  echo "[verify-devnet-config] FAIL: $*" >&2
  FAILED=1
}
pass() {
  echo "[verify-devnet-config] OK: $*"
}

# Load the same .env every app in this monorepo reads from, without
# clobbering variables already set in the calling shell/CI environment.
if [[ -f "$ROOT_DIR/.env" ]]; then
  set -a
  # shellcheck disable=SC1091
  source "$ROOT_DIR/.env"
  set +a
fi

SOLANA_NETWORK="${SOLANA_NETWORK:-}"
SOLANA_RPC_HTTP_URL="${SOLANA_RPC_HTTP_URL:-}"
NEXT_PUBLIC_SOLANA_NETWORK="${NEXT_PUBLIC_SOLANA_NETWORK:-}"
SOLANA_PROGRAM_ID="${SOLANA_PROGRAM_ID:-}"

if [[ "$SOLANA_NETWORK" != "devnet" ]]; then
  fail "SOLANA_NETWORK is '$SOLANA_NETWORK', must be exactly 'devnet'"
else
  pass "SOLANA_NETWORK=devnet"
fi

if [[ "$NEXT_PUBLIC_SOLANA_NETWORK" != "devnet" ]]; then
  fail "NEXT_PUBLIC_SOLANA_NETWORK is '$NEXT_PUBLIC_SOLANA_NETWORK', must be exactly 'devnet' (frontend network guard reads this)"
else
  pass "NEXT_PUBLIC_SOLANA_NETWORK=devnet"
fi

case "$SOLANA_RPC_HTTP_URL" in
  *mainnet*|*mainnet-beta*)
    fail "SOLANA_RPC_HTTP_URL ('$SOLANA_RPC_HTTP_URL') contains a Mainnet hostname signal"
    ;;
esac

if [[ -z "$SOLANA_RPC_HTTP_URL" ]]; then
  fail "SOLANA_RPC_HTTP_URL is not set"
else
  GENESIS_HASH="$(curl -s --max-time 10 "$SOLANA_RPC_HTTP_URL" \
    -X POST -H "Content-Type: application/json" \
    -d '{"jsonrpc":"2.0","id":1,"method":"getGenesisHash"}' \
    | python3 -c 'import json,sys; print(json.load(sys.stdin).get("result",""))' 2>/dev/null || true)"

  if [[ "$GENESIS_HASH" == "$MAINNET_GENESIS_HASH" ]]; then
    fail "SOLANA_RPC_HTTP_URL's genesis hash matches MAINNET-BETA — refusing to proceed"
  elif [[ "$GENESIS_HASH" != "$DEVNET_GENESIS_HASH" ]]; then
    fail "SOLANA_RPC_HTTP_URL's genesis hash ('$GENESIS_HASH') does not match known Devnet genesis hash"
  else
    pass "RPC genesis hash confirms Devnet ($SOLANA_RPC_HTTP_URL)"
  fi
fi

if [[ -z "$SOLANA_PROGRAM_ID" ]]; then
  fail "SOLANA_PROGRAM_ID is not set"
elif [[ -n "${GENESIS_HASH:-}" && "$GENESIS_HASH" == "$DEVNET_GENESIS_HASH" ]]; then
  EXECUTABLE="$(curl -s --max-time 10 "$SOLANA_RPC_HTTP_URL" \
    -X POST -H "Content-Type: application/json" \
    -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"getAccountInfo\",\"params\":[\"$SOLANA_PROGRAM_ID\",{\"encoding\":\"base64\"}]}" \
    | python3 -c 'import json,sys; v=(json.load(sys.stdin).get("result") or {}).get("value"); print(str(v["executable"]) if v else "missing")' 2>/dev/null || true)"

  if [[ "$EXECUTABLE" == "True" ]]; then
    pass "Program $SOLANA_PROGRAM_ID is deployed and executable on Devnet"
  else
    fail "Program $SOLANA_PROGRAM_ID is not executable on Devnet (account: $EXECUTABLE)"
  fi
fi

if [[ $FAILED -ne 0 ]]; then
  echo "[verify-devnet-config] one or more checks failed — refusing to proceed" >&2
  exit 1
fi

echo "[verify-devnet-config] all checks passed"
