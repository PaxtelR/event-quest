# Build context is the repo root (pnpm workspace: apps/web depends on
# packages/chain-client via a workspace symlink, so `pnpm install` needs
# the whole repo, not just apps/web's own directory).
FROM node:22-slim AS builder

RUN corepack enable
WORKDIR /workspace
COPY . .
RUN pnpm install --frozen-lockfile
# NEXT_PUBLIC_* vars are inlined into the client bundle at build time, not
# read at runtime — Railway (and most platforms) exposes a service's env
# vars to its own build step automatically, so set these as real variables
# on the web service, not just at deploy/runtime.
RUN pnpm --filter=./apps/web build

FROM node:22-slim AS runtime
WORKDIR /app
ENV NODE_ENV=production
ENV PORT=3000
# Next's standalone server.js does `process.env.HOSTNAME || '0.0.0.0'` —
# Docker (and platforms like Railway) auto-populate `HOSTNAME` with the
# container's own id/hostname, so without this override the server binds
# to that specific hostname instead of all interfaces, and an external
# healthcheck can never reach it even though the process is running fine.
ENV HOSTNAME=0.0.0.0

# `output: "standalone"` (apps/web/next.config.ts) traces a self-contained
# server.js + the minimal node_modules subset it actually needs, mirroring
# the monorepo path — running it from /app as `node apps/web/server.js` is
# the standard invocation for a standalone build out of a workspace.
COPY --from=builder /workspace/apps/web/.next/standalone ./
COPY --from=builder /workspace/apps/web/.next/static ./apps/web/.next/static

EXPOSE 3000
CMD ["node", "apps/web/server.js"]
