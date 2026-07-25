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
