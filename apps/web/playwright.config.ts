import { defineConfig, devices } from "@playwright/test";

// Phase 5 local E2E (spec §25.4). Orchestrated by scripts/e2e-local.sh,
// which starts Surfpool + apps/api + apps/indexer + a production apps/web
// build *before* invoking `playwright test` — this config doesn't manage
// any of those servers itself (no `webServer` entry) since a single
// vertical scenario here spans a whole real stack, not just this app.
export default defineConfig({
  testDir: "./tests/e2e",
  timeout: 60_000,
  expect: { timeout: 15_000 },
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
