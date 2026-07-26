import path from "node:path";
import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    environment: "jsdom",
    globals: true,
    setupFiles: ["./src/test/setup.ts"],
    // Unit tests live under src/**/*.test.ts; tests/ is exclusively
    // Playwright's (tests/e2e, tests/e2e-devnet) — Vitest's default include
    // glob also matches *.spec.ts, which collides with Playwright's own
    // `test()` global if left unexcluded.
    exclude: ["**/node_modules/**", "tests/**"],
  },
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },
});
