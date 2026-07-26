# Build context is the repo root (see docker-compose.yml / railway.api.json) —
# this is a Cargo workspace, so `cargo build -p eventquest-api` needs every
# workspace member's Cargo.toml present to resolve, even though only
# eventquest-api's own dependency graph actually gets compiled.
FROM rust:1.97-slim-bookworm AS builder

RUN apt-get update \
    && apt-get install -y --no-install-recommends build-essential pkg-config \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /workspace
COPY Cargo.toml Cargo.lock ./
COPY programs programs
COPY apps apps
COPY crates crates
# crates/eventquest-chain's `anchor_lang::declare_program!` reads this IDL
# at compile time (not runtime) — without it, that crate fails to build.
COPY idls idls

# `sqlx::migrate!("./migrations")` embeds migration SQL into the binary at
# compile time — the runtime image below never needs the migrations
# directory on disk.
RUN cargo build --release --locked -p eventquest-api

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --create-home --shell /usr/sbin/nologin eventquest

WORKDIR /app
COPY --from=builder /workspace/target/release/eventquest-api /usr/local/bin/eventquest-api

# The attestor keypair is a local file everywhere else (docker-compose,
# scripts/e2e-local.sh, scripts/devnet-e2e.sh) — a stateless platform like
# Railway has no equivalent of "just put a file on disk" without a Volume,
# so this materializes it from an env var instead when one is supplied.
# `ATTESTOR_KEYPAIR_PATH` still works unchanged if you mount a Volume and
# point it at a real file — this is additive, not a replacement.
COPY infrastructure/docker/entrypoint-api.sh /usr/local/bin/entrypoint-api.sh
RUN chmod +x /usr/local/bin/entrypoint-api.sh && chown eventquest:eventquest /app

USER eventquest
EXPOSE 3001
ENTRYPOINT ["/usr/local/bin/entrypoint-api.sh"]
