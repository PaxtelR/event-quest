# Build context is the repo root — see api.Dockerfile's header comment for
# why the whole Cargo workspace needs to be copied in.
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

RUN cargo build --release --locked -p eventquest-indexer

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --create-home --shell /usr/sbin/nologin eventquest

WORKDIR /app
COPY --from=builder /workspace/target/release/eventquest-indexer /usr/local/bin/eventquest-indexer

USER eventquest
# No exposed port — this is a background worker, not an HTTP service.
# On Railway, deploy it as a service with no public networking / no
# healthcheck path (or a Railway "worker" service type, if available).
ENTRYPOINT ["eventquest-indexer"]
