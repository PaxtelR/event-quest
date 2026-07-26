import path from "node:path";
import { config as loadEnv } from "dotenv";
import type { NextConfig } from "next";

// The monorepo keeps one shared `.env` at the repo root (same file
// apps/api/apps/indexer read via `dotenvy::dotenv()` from the workspace
// root) rather than a duplicate `apps/web/.env.local` that could drift.
// Loaded here, synchronously, before Next.js's own NEXT_PUBLIC_ inlining
// scans `process.env`.
loadEnv({ path: path.resolve(process.cwd(), "../../.env") });

const apiUrl = process.env.API_URL ?? "http://localhost:3001";

const nextConfig: NextConfig = {
  reactStrictMode: true,
  // `next start`'s default gzip compression buffers responses to build
  // deflate blocks — harmless for normal pages, but it silently breaks the
  // rewritten `/api/v1/public/checkpoints/:id/qr-stream` SSE endpoint: a
  // browser (which always sends `Accept-Encoding: gzip`) never receives a
  // single event because nothing ever flushes on an SSE connection that's
  // deliberately kept open. apps/api doesn't compress its own responses
  // either, so nothing downstream needs this.
  compress: false,
  // Proxies API calls through this app's own origin so the session cookie
  // apps/api issues (HttpOnly, Secure, SameSite=Lax — spec §9.1) is a
  // same-origin cookie from the browser's point of view. Simpler and more
  // robust than CORS + SameSite=None for a browser session flow.
  async rewrites() {
    return [
      { source: "/api/v1/:path*", destination: `${apiUrl}/api/v1/:path*` },
      { source: "/health/:path*", destination: `${apiUrl}/health/:path*` },
    ];
  },
};

export default nextConfig;
