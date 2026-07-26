import path from "node:path";
import { fileURLToPath } from "node:url";
import { expect, test, type Page } from "@playwright/test";

// Phase 5 (spec §25.4): the full vertical scenario against a real local
// Surfnet (scripts/e2e-local.sh) — create event → checkpoint → activate →
// QR appears → auth → scan → sign → confirm → points/dashboard update →
// duplicate rejected. Every signature and transaction here is real (see
// inject-test-wallet.js); only the cluster is local instead of public
// Devnet.

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const TEST_WALLET_SCRIPT = path.join(__dirname, "inject-test-wallet.js");
const RPC_URL = process.env.E2E_RPC_URL ?? "http://127.0.0.1:8899";

async function rpcCall<T>(method: string, params: unknown[]): Promise<T> {
  const response = await fetch(RPC_URL, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ jsonrpc: "2.0", id: 1, method, params }),
  });
  const json = await response.json();
  if (json.error) throw new Error(`${method} failed: ${json.error.message}`);
  return json.result as T;
}

async function airdrop(address: string, lamports = 5_000_000_000) {
  const signature = await rpcCall<string>("requestAirdrop", [address, lamports]);
  for (let attempt = 0; attempt < 40; attempt++) {
    const statuses = await rpcCall<{ value: Array<{ confirmationStatus?: string } | null> }>(
      "getSignatureStatuses",
      [[signature]],
    );
    const status = statuses.value[0];
    if (status?.confirmationStatus === "confirmed" || status?.confirmationStatus === "finalized") {
      return;
    }
    await new Promise((resolve) => setTimeout(resolve, 500));
  }
  throw new Error(`airdrop to ${address} did not confirm in time`);
}

/** Connects the injected E2E test wallet and completes the sign-in flow
 * (nonce + real message signature + /auth/verify), returning its address. */
async function connectAndSignIn(page: Page): Promise<string> {
  await page.getByRole("button", { name: "Connect wallet" }).click();
  await page.getByRole("menuitem", { name: "E2E Test Wallet" }).click();
  // "Connect wallet" disappears the instant a wallet is *selected* —
  // before the async nonce/sign/verify round trip that actually
  // establishes the session even starts. Waiting for "Disconnect" (only
  // rendered once `isSignedIn` is true) is what actually proves sign-in
  // completed.
  await expect(page.getByRole("button", { name: "Disconnect" })).toBeVisible({ timeout: 20_000 });
  const address = await page.evaluate(() => (window as unknown as Record<string, string>).__E2E_WALLET_ADDRESS__);
  if (!address) throw new Error("wallet did not report its address after sign-in");
  return address;
}

/** `<input type="datetime-local">` wants `YYYY-MM-DDTHH:mm` in local time. */
function toDateTimeLocal(date: Date): string {
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}T${pad(date.getHours())}:${pad(date.getMinutes())}`;
}

test("full vertical scenario: event through duplicate check-in rejection", async ({ browser, baseURL }) => {
  test.setTimeout(180_000);

  const organizerContext = await browser.newContext();
  await organizerContext.addInitScript({ path: TEST_WALLET_SCRIPT });
  const organizerPage = await organizerContext.newPage();

  const participantContext = await browser.newContext();
  await participantContext.addInitScript({ path: TEST_WALLET_SCRIPT });
  const participantPage = await participantContext.newPage();

  // --- Organizer connects and is funded --------------------------------
  await organizerPage.goto("/admin");
  const organizerAddress = await connectAndSignIn(organizerPage);
  await airdrop(organizerAddress);

  // --- Create event ------------------------------------------------------
  await organizerPage.goto("/admin/events/new");
  const eventName = `E2E Event ${Date.now()}`;
  const now = new Date();
  const startsAt = new Date(now.getTime() - 60_000);
  const endsAt = new Date(now.getTime() + 3_600_000);

  await organizerPage.getByLabel("Event name").fill(eventName);
  await organizerPage.getByLabel("Starts at").fill(toDateTimeLocal(startsAt));
  await organizerPage.getByLabel("Ends at").fill(toDateTimeLocal(endsAt));
  await organizerPage.getByRole("button", { name: "Create event" }).click();
  await organizerPage.waitForURL(/\/admin\/events\/[^/]+$/);

  // --- Publish (real organizer-signed on-chain transaction) -------------
  await organizerPage.getByRole("button", { name: "Publish" }).click();
  await expect(organizerPage.getByText("Active", { exact: true })).toBeVisible({ timeout: 45_000 });

  // --- Create + activate a checkpoint ------------------------------------
  await organizerPage.getByRole("link", { name: "Manage checkpoints" }).click();
  await organizerPage.waitForURL(/\/checkpoints$/);

  const checkpointName = `E2E Checkpoint ${Date.now()}`;
  const opensAt = new Date(now.getTime() - 60_000);
  const closesAt = new Date(now.getTime() + 3_600_000);
  await organizerPage.getByLabel("Checkpoint name").fill(checkpointName);
  await organizerPage.getByLabel("Opens at").fill(toDateTimeLocal(opensAt));
  await organizerPage.getByLabel("Closes at").fill(toDateTimeLocal(closesAt));
  await organizerPage.getByRole("button", { name: "Add checkpoint" }).click();
  await expect(organizerPage.getByText(checkpointName)).toBeVisible({ timeout: 15_000 });

  await organizerPage.getByRole("button", { name: "Activate" }).click();
  await expect(organizerPage.getByText("Active", { exact: true }).first()).toBeVisible({ timeout: 45_000 });

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

  await expect(organizerPage.locator("img")).toBeVisible({ timeout: 15_000 });

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
  await airdrop(participantAddress);

  const firstQrToken = await fetchCurrentQrToken();
  await participantPage.goto(`/check-in?token=${encodeURIComponent(firstQrToken)}`);
  await participantPage.waitForURL(/\/check-in\/result\?grantId=/, { timeout: 60_000 });
  await expect(participantPage.getByText("Attendance confirmed on Solana Devnet.")).toBeVisible({
    timeout: 30_000,
  });

  // --- Dashboard/points update: the organizer sees the real check-in -----
  await organizerPage.goto("/admin/events");
  await organizerPage.getByText(eventName).click();
  await organizerPage.getByRole("link", { name: "Participants" }).click();
  await expect(organizerPage.getByText("10 points")).toBeVisible({ timeout: 15_000 });

  // --- Duplicate check-in must be rejected --------------------------------
  const secondQrToken = await fetchCurrentQrToken();
  await participantPage.goto(`/check-in?token=${encodeURIComponent(secondQrToken)}`);
  await expect(participantPage.getByRole("alert")).toBeVisible({ timeout: 30_000 });
  await expect(participantPage).not.toHaveURL(/\/check-in\/result/);
});
