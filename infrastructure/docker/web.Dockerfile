# Build context is the repo root (pnpm workspace: apps/web depends on
# packages/chain-client via a workspace symlink, so `pnpm install` needs
# the whole repo, not just apps/web's own directory).
FROM node:22-slim AS builder

# Docker never exposes a platform's service variables to `RUN` steps
# automatically — only variables explicitly declared with `ARG` here get
# populated from the `--build-arg`s Railway (or any platform) passes in.
# Both `API_URL` (next.config.ts's rewrite destination — resolved once,
# during `next build`, and baked into the standalone output, never read
# again at container start) and every `NEXT_PUBLIC_*` var (inlined into
# the client bundle the same way) need this, or they silently fall back
# to their dev defaults no matter what's set in the platform's dashboard.
ARG API_URL
ARG NEXT_PUBLIC_DEFAULT_LOCALE
ARG NEXT_PUBLIC_THEME
ARG NEXT_PUBLIC_ACCENT_COLOR
ARG NEXT_PUBLIC_SOLANA_NETWORK
ARG NEXT_PUBLIC_SOLANA_PROGRAM_ID
ENV API_URL=$API_URL
ENV NEXT_PUBLIC_DEFAULT_LOCALE=$NEXT_PUBLIC_DEFAULT_LOCALE
ENV NEXT_PUBLIC_THEME=$NEXT_PUBLIC_THEME
ENV NEXT_PUBLIC_ACCENT_COLOR=$NEXT_PUBLIC_ACCENT_COLOR
ENV NEXT_PUBLIC_SOLANA_NETWORK=$NEXT_PUBLIC_SOLANA_NETWORK
ENV NEXT_PUBLIC_SOLANA_PROGRAM_ID=$NEXT_PUBLIC_SOLANA_PROGRAM_ID

RUN corepack enable
WORKDIR /workspace
COPY . .
RUN pnpm install --frozen-lockfile
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
