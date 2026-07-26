import path from "node:path";
import { fileURLToPath } from "node:url";
import { expect, test, type Page } from "@playwright/test";

import { fundDevnetWallet } from "./fund-devnet-wallet";

// Spec §25.6: the same vertical scenario as
// apps/web/tests/e2e/vertical-scenario.spec.ts (Phase 5), but against real
// Devnet — real frontend, real apps/api, real Postgres/Redis, the real
// deployed eventquest program, a real indexer, and a real (faucet-free —
// see fund-devnet-wallet.ts) test wallet. Orchestrated by
// scripts/devnet-e2e.sh, which points every service at Devnet before
// invoking this config.

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const TEST_WALLET_SCRIPT = path.join(__dirname, "../e2e/inject-test-wallet.js");
const RPC_URL_OVERRIDE = process.env.E2E_RPC_URL ?? "https://api.devnet.solana.com";

async function connectAndSignIn(page: Page): Promise<string> {
  await page.getByRole("button", { name: "Connect wallet" }).click();
  await page.getByRole("menuitem", { name: "E2E Test Wallet" }).click();
  // Waiting for "Connect wallet" to disappear only proves a wallet was
  // *selected* — it vanishes before the async nonce/sign/verify round trip
  // that actually establishes the session even starts (see Phase 5's
  // vertical-scenario.spec.ts for the debugging history behind this).
  // "Disconnect" only renders once sign-in genuinely completed.
  await expect(page.getByRole("button", { name: "Disconnect" })).toBeVisible({ timeout: 30_000 });
  const address = await page.evaluate(() => (window as unknown as Record<string, string>).__E2E_WALLET_ADDRESS__);
  if (!address) throw new Error("wallet did not report its address after sign-in");
  return address;
}

function toDateTimeLocal(date: Date): string {
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}T${pad(date.getHours())}:${pad(date.getMinutes())}`;
}

async function addRpcOverride(context: Awaited<ReturnType<import("@playwright/test").Browser["newContext"]>>): Promise<void> {
  await context.addInitScript((rpcUrl) => {
    (window as unknown as Record<string, string>).__E2E_RPC_URL_OVERRIDE__ = rpcUrl;
  }, RPC_URL_OVERRIDE);
}

test("full vertical scenario on real Devnet: event through duplicate check-in rejection", async ({ browser, baseURL }) => {
  test.setTimeout(300_000);

  const organizerContext = await browser.newContext();
  await addRpcOverride(organizerContext);
  await organizerContext.addInitScript({ path: TEST_WALLET_SCRIPT });
  const organizerPage = await organizerContext.newPage();

  const participantContext = await browser.newContext();
  await addRpcOverride(participantContext);
  await participantContext.addInitScript({ path: TEST_WALLET_SCRIPT });
  const participantPage = await participantContext.newPage();

  // --- Organizer connects and is funded (direct transfer, not faucet) ----
  await organizerPage.goto("/admin");
  const organizerAddress = await connectAndSignIn(organizerPage);
  const organizerFundingSignature = await fundDevnetWallet(organizerAddress, 20_000_000n);
  console.log("organizer funded:", organizerFundingSignature);

  // --- Create event ------------------------------------------------------
  await organizerPage.goto("/admin/events/new");
  const eventName = `Devnet E2E Event ${Date.now()}`;
  const now = new Date();
  const startsAt = new Date(now.getTime() - 60_000);
  const endsAt = new Date(now.getTime() + 3_600_000);

  await organizerPage.getByLabel("Event name").fill(eventName);
  await organizerPage.getByLabel("Starts at").fill(toDateTimeLocal(startsAt));
  await organizerPage.getByLabel("Ends at").fill(toDateTimeLocal(endsAt));
  await organizerPage.getByRole("button", { name: "Create event" }).click();
  await organizerPage.waitForURL(/\/admin\/events\/[^/]+$/, { timeout: 60_000 });

  // --- Publish (real organizer-signed on-chain Devnet transaction) -------
  await organizerPage.getByRole("button", { name: "Publish" }).click();
  await expect(organizerPage.getByText("Active", { exact: true })).toBeVisible({ timeout: 90_000 });

  // --- Create + activate a checkpoint ------------------------------------
  await organizerPage.getByRole("link", { name: "Manage checkpoints" }).click();
  await organizerPage.waitForURL(/\/checkpoints$/);

  const checkpointName = `Devnet E2E Checkpoint ${Date.now()}`;
  const opensAt = new Date(now.getTime() - 60_000);
  const closesAt = new Date(now.getTime() + 3_600_000);
  await organizerPage.getByLabel("Checkpoint name").fill(checkpointName);
  await organizerPage.getByLabel("Opens at").fill(toDateTimeLocal(opensAt));
  await organizerPage.getByLabel("Closes at").fill(toDateTimeLocal(closesAt));
  await organizerPage.getByRole("button", { name: "Add checkpoint" }).click();
  await expect(organizerPage.getByText(checkpointName)).toBeVisible({ timeout: 30_000 });

  await organizerPage.getByRole("button", { name: "Activate" }).click();
  await expect(organizerPage.getByText("Active", { exact: true }).first()).toBeVisible({ timeout: 90_000 });

  // --- Open the QR display and capture the current token -----------------
  const displayTokenResponsePromise = organizerPage.waitForResponse(
    (response) => response.url().includes("/display-token") && response.request().method() === "POST",
  );
  await organizerPage.getByRole("link", { name: "Open QR display" }).click();
  const displayTokenResponse = await displayTokenResponsePromise;
  const { displayAccessToken } = (await displayTokenResponse.json()) as { displayAccessToken: string };
  await organizerPage.waitForURL(/\/admin\/checkpoints\/[^/]+\/display$/);
  const checkpointId = organizerPage.url().match(/checkpoints\/([^/]+)\/display/)?.[1];
  if (!checkpointId) throw new Error("could not extract checkpointId from the display page URL");

  await expect(organizerPage.locator("img")).toBeVisible({ timeout: 30_000 });

  async function fetchCurrentQrToken(): Promise<string> {
    const response = await fetch(
      `${baseURL}/api/v1/public/checkpoints/${checkpointId}/current-qr?displayAccessToken=${encodeURIComponent(displayAccessToken)}`,
    );
    const body = (await response.json()) as { token: string };
    return body.token;
  }

  // --- Participant connects, is funded, then checks in --------------------
  await participantPage.goto("/check-in");
  const participantAddress = await connectAndSignIn(participantPage);
  const participantFundingSignature = await fundDevnetWallet(participantAddress, 10_000_000n);
  console.log("participant funded:", participantFundingSignature);

  const firstQrToken = await fetchCurrentQrToken();
  await participantPage.goto(`/check-in?token=${encodeURIComponent(firstQrToken)}`);
  await participantPage.waitForURL(/\/check-in\/result\?grantId=/, { timeout: 120_000 });
  await expect(participantPage.getByText("Attendance confirmed on Solana Devnet.")).toBeVisible({
    timeout: 90_000,
  });

  // --- Dashboard/points update: the organizer sees the real check-in, and
  // apps/indexer has really processed the real AttendanceRecorded event --
  await organizerPage.goto("/admin/events");
  await organizerPage.getByText(eventName).click();
  await organizerPage.getByRole("link", { name: "Participants" }).click();
  await expect(organizerPage.getByText("10 points")).toBeVisible({ timeout: 30_000 });

  // --- Duplicate check-in must be rejected --------------------------------
  const secondQrToken = await fetchCurrentQrToken();
  await participantPage.goto(`/check-in?token=${encodeURIComponent(secondQrToken)}`);
  await expect(participantPage.getByRole("alert")).toBeVisible({ timeout: 60_000 });
  await expect(participantPage).not.toHaveURL(/\/check-in\/result/);
});
