#!/bin/sh
# Materializes the attestor keypair from ATTESTOR_KEYPAIR_JSON (a raw
# solana-keygen JSON byte array, e.g. `cat attestor-devnet.json`) into a
# file on the container's own writable layer, then runs eventquest-api
# with ATTESTOR_KEYPAIR_PATH pointed at it — for platforms like Railway
# with no persistent local file to reference otherwise. If
# ATTESTOR_KEYPAIR_PATH is already set to a real (e.g. Volume-mounted)
# file, this does nothing and that path is used unchanged.
set -eu

if [ -n "${ATTESTOR_KEYPAIR_JSON:-}" ]; then
  KEYPAIR_FILE="/app/attestor-keypair.json"
  # Default umask would leave this world-readable within the container —
  # cheap defense in depth for a file holding a private key in plaintext.
  ( umask 077 && echo "$ATTESTOR_KEYPAIR_JSON" > "$KEYPAIR_FILE" )
  export ATTESTOR_KEYPAIR_PATH="$KEYPAIR_FILE"
fi

exec eventquest-api
