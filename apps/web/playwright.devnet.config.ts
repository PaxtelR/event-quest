import { defineConfig, devices } from "@playwright/test";

// Spec §25.6: the browser E2E scenario run against real Devnet — real
// frontend, real apps/api, real Postgres/Redis, the real deployed program,
// a real (if faucet-free — see fund-devnet-wallet.ts) test wallet, and a
// real indexer. Orchestrated by scripts/devnet-e2e.sh. Kept as a separate
// config/testDir from playwright.config.ts (Phase 5's local-Surfnet suite)
// so a plain `pnpm test:e2e` never accidentally spends real Devnet SOL or
// depends on public RPC latency.
export default defineConfig({
  testDir: "./tests/e2e-devnet",
  // Public Devnet RPC confirmation is far slower and less predictable than
  // a local Surfnet — generous timeouts are the honest choice here, not a
  // band-aid over a bug.
  timeout: 180_000,
  expect: { timeout: 60_000 },
  fullyParallel: false,
  workers: 1,
  retries: 0,
  reporter: [["list"]],
  use: {
    baseURL: process.env.E2E_BASE_URL ?? "http://localhost:3100",
    trace: "retain-on-failure",
    screenshot: "only-on-failure",
  },
  projects: [{ name: "chromium", use: { ...devices["Desktop Chrome"] } }],
});
