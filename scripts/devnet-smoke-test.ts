#!/usr/bin/env -S node --experimental-strip-types
// Spec §0.9's mandatory Devnet smoke test — the real flow, no mocks, no
// local validator: confirm Devnet, confirm the program is executable, fund
// fresh test wallets, create an event + checkpoint, join, check in with a
// real participant+attestor co-signed transaction, read back the
// Attendance PDA, then prove a duplicate check-in is rejected on-chain.
// Talks to the deployed program directly through @eventquest/chain-client
// (the same Codama-generated client apps/api and apps/web use) rather than
// through the live HTTP API — a "backend equivalent" challenge hash (spec
// step 6) is generated locally since the program itself never validates
// QR/JWT contents, only that challengeHash is non-zero (see
// programs/eventquest/src/instructions/check_in.rs).
//
// Exits non-zero if any step fails or if the duplicate check-in is NOT
// rejected. Prints every signature/PDA/pubkey needed for
// docs/DEVNET_TEST_REPORT.md — never a private key.

import { createHash, randomBytes } from "node:crypto";
import { readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  address,
  appendTransactionMessageInstructions,
  createKeyPairSignerFromBytes,
  createSolanaRpc,
  createSolanaRpcSubscriptions,
  createTransactionMessage,
  fetchEncodedAccount,
  generateKeyPairSigner,
  getSignatureFromTransaction,
  lamports,
  pipe,
  sendAndConfirmTransactionFactory,
  setTransactionMessageFeePayerSigner,
  setTransactionMessageLifetimeUsingBlockhash,
  signTransactionMessageWithSigners,
  type Address,
  type KeyPairSigner,
} from "@solana/kit";
import { getTransferSolInstruction } from "@solana-program/system";
import {
  EVENTQUEST_PROGRAM_ADDRESS,
  EventStatus,
  fetchAttendanceAccount,
  findAttendancePda,
  findCheckpointPda,
  findEventPda,
  findParticipantEventPda,
  getCheckInInstructionAsync,
  getCreateCheckpointInstructionAsync,
  getInitializeEventInstructionAsync,
  getJoinEventInstructionAsync,
  getUpdateEventStatusInstruction,
} from "@eventquest/chain-client";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const ROOT_DIR = path.resolve(__dirname, "..");

const DEVNET_GENESIS_HASH = "EtWTRABZaYq6iMfeYKouRu166VU2xqa1wcaWoxPkrZBG";
const MAINNET_GENESIS_HASH = "5eykt4UsFv8P8NJdTREpY1vzqKqZKvdpKuc147dw2N9d";
const EXPLORER_BASE = "https://explorer.solana.com";

type StepResult = { step: string; detail: string };
const results: StepResult[] = [];
function record(step: string, detail: string): void {
  results.push({ step, detail });
  console.log(`[devnet-smoke-test] ${step}: ${detail}`);
}
function explorerTx(signature: string): string {
  return `${EXPLORER_BASE}/tx/${signature}?cluster=devnet`;
}
function explorerAddress(addr: string): string {
  return `${EXPLORER_BASE}/address/${addr}?cluster=devnet`;
}

async function loadEnv(): Promise<void> {
  const envPath = path.join(ROOT_DIR, ".env");
  try {
    // Node 20.6+ built-in — avoids taking on a `dotenv` dependency just for
    // this one script. Never overrides a variable already set by the
    // calling shell/CI.
    process.loadEnvFile(envPath);
  } catch {
    // No .env file (e.g. CI injects env vars directly) — fine.
  }
}

async function loadAttestorSigner(): Promise<KeyPairSigner> {
  const keypairPath = process.env.ATTESTOR_KEYPAIR_PATH ?? ".secrets/attestor-devnet.json";
  const resolvedPath = path.isAbsolute(keypairPath) ? keypairPath : path.join(ROOT_DIR, keypairPath);
  const raw = JSON.parse(await readFile(resolvedPath, "utf8")) as number[];
  return createKeyPairSignerFromBytes(new Uint8Array(raw));
}

function sha256(input: string): Uint8Array {
  return new Uint8Array(createHash("sha256").update(input).digest());
}

async function main(): Promise<void> {
  await loadEnv();

  const network = process.env.SOLANA_NETWORK ?? "";
  const rpcHttpUrl = process.env.SOLANA_RPC_HTTP_URL ?? "https://api.devnet.solana.com";
  const rpcWsUrl = process.env.SOLANA_RPC_WS_URL ?? "wss://api.devnet.solana.com";

  if (network !== "devnet") {
    throw new Error(`SOLANA_NETWORK is '${network}', refusing to run the smoke test against anything but 'devnet'`);
  }

  const rpc = createSolanaRpc(rpcHttpUrl);
  const rpcSubscriptions = createSolanaRpcSubscriptions(rpcWsUrl);
  const sendAndConfirm = sendAndConfirmTransactionFactory({ rpc, rpcSubscriptions });

  async function signAndSend(instructions: Parameters<typeof appendTransactionMessageInstructions>[0], feePayer: KeyPairSigner): Promise<string> {
    const { value: latestBlockhash } = await rpc.getLatestBlockhash().send();
    const message = pipe(
      createTransactionMessage({ version: 0 }),
      (tx) => setTransactionMessageFeePayerSigner(feePayer, tx),
      (tx) => setTransactionMessageLifetimeUsingBlockhash(latestBlockhash, tx),
      (tx) => appendTransactionMessageInstructions(instructions, tx),
    );
    const signedTx = await signTransactionMessageWithSigners(message);
    const signature = getSignatureFromTransaction(signedTx);
    // See apps/web/tests/e2e-devnet/fund-devnet-wallet.ts for why this cast
    // is needed — @solana/kit's pipe() chain doesn't narrow the lifetime
    // constraint type on its own.
    await sendAndConfirm(signedTx as Parameters<typeof sendAndConfirm>[0], { commitment: "confirmed" });
    return signature;
  }

  // --- Step 1: confirm the RPC is really Devnet --------------------------
  const genesisHash = await rpc.getGenesisHash().send();
  if (genesisHash === MAINNET_GENESIS_HASH) {
    throw new Error("RPC genesis hash matches MAINNET-BETA — refusing to proceed");
  }
  if (genesisHash !== DEVNET_GENESIS_HASH) {
    throw new Error(`RPC genesis hash '${genesisHash}' does not match known Devnet genesis hash`);
  }
  record("1. RPC is Devnet", `genesisHash=${genesisHash} rpc=${rpcHttpUrl}`);

  // --- Step 2: confirm the program is deployed and executable ------------
  const programAccount = await fetchEncodedAccount(rpc, EVENTQUEST_PROGRAM_ADDRESS);
  if (!programAccount.exists || !programAccount.executable) {
    throw new Error(`Program ${EVENTQUEST_PROGRAM_ADDRESS} is not executable on Devnet`);
  }
  record("2. Program executable", `programId=${EVENTQUEST_PROGRAM_ADDRESS}`);

  // --- Step 3: fund fresh test wallets ------------------------------------
  // The attestor devnet keypair is already funded from earlier phases —
  // funding fresh organizer/participant keypairs from it directly avoids
  // the public faucet's aggressive rate limiting entirely.
  const attestor = await loadAttestorSigner();
  const organizer = await generateKeyPairSigner();
  const participant = await generateKeyPairSigner();

  const attestorBalance = await rpc.getBalance(attestor.address).send();
  const ORGANIZER_FUNDING = 20_000_000n; // 0.02 SOL
  const PARTICIPANT_FUNDING = 10_000_000n; // 0.01 SOL
  const FUNDING_BUFFER = 5_000_000n; // leaves the attestor keypair usable for future runs
  const required = ORGANIZER_FUNDING + PARTICIPANT_FUNDING + FUNDING_BUFFER;
  if (attestorBalance.value < required) {
    throw new Error(
      `attestor devnet keypair (${attestor.address}) has ${attestorBalance.value} lamports, needs at least ${required} to fund test wallets — top it up via 'solana airdrop' first`,
    );
  }

  const fundingSignature = await signAndSend(
    [
      getTransferSolInstruction({ source: attestor, destination: organizer.address, amount: lamports(ORGANIZER_FUNDING) }),
      getTransferSolInstruction({ source: attestor, destination: participant.address, amount: lamports(PARTICIPANT_FUNDING) }),
    ],
    attestor,
  );
  record(
    "3. Test wallets funded",
    `organizer=${organizer.address} participant=${participant.address} attestor=${attestor.address} tx=${fundingSignature}`,
  );

  // --- Step 4: create the event -------------------------------------------
  const runNonce = Date.now();
  const externalIdHash = sha256(`eventquest-smoke-test-event-${runNonce}`);
  const [eventPda] = await findEventPda({ authority: organizer.address, externalIdHash });

  const nowSeconds = Math.floor(Date.now() / 1000);
  const startsAt = nowSeconds - 60;
  const endsAt = nowSeconds + 3600;

  const initializeEventIx = await getInitializeEventInstructionAsync({
    authority: organizer,
    externalIdHash,
    startsAt,
    endsAt,
  });
  const createEventSignature = await signAndSend([initializeEventIx], organizer);
  record("4. Event created", `eventPda=${eventPda} tx=${createEventSignature}`);

  const activateIx = getUpdateEventStatusInstruction({
    authority: organizer,
    event: eventPda,
    newStatus: EventStatus.Active,
  });
  const activateEventSignature = await signAndSend([activateIx], organizer);
  record("4b. Event activated", `tx=${activateEventSignature}`);

  // --- Step 5: create the checkpoint ---------------------------------------
  const checkpointExternalIdHash = sha256(`eventquest-smoke-test-checkpoint-${runNonce}`);
  const [checkpointPda] = await findCheckpointPda({ event: eventPda, externalIdHash: checkpointExternalIdHash });
  const opensAt = nowSeconds - 60;
  const closesAt = nowSeconds + 3600;
  const points = 10;

  const createCheckpointIx = await getCreateCheckpointInstructionAsync({
    authority: organizer,
    event: eventPda,
    externalIdHash: checkpointExternalIdHash,
    attestor: attestor.address,
    opensAt,
    closesAt,
    points,
  });
  const createCheckpointSignature = await signAndSend([createCheckpointIx], organizer);
  record("5. Checkpoint created", `checkpointPda=${checkpointPda} attestor=${attestor.address} points=${points} tx=${createCheckpointSignature}`);

  // Participant must join before checking in (check_in requires an
  // already-existing ParticipantEventAccount — see join_event.rs).
  const joinEventIx = await getJoinEventInstructionAsync({ participant, event: eventPda });
  const joinEventSignature = await signAndSend([joinEventIx], participant);
  const [participantEventPda] = await findParticipantEventPda({ event: eventPda, participant: participant.address });
  record("5b. Participant joined", `participantEventPda=${participantEventPda} tx=${joinEventSignature}`);

  // --- Step 6: generate a challenge equivalent to the backend flow --------
  // The on-chain program only requires a non-zero 32-byte challenge hash
  // (see check_in_handler) — the actual QR/JWT validation is an off-chain
  // (apps/api) concern. This mirrors that shape: a random per-check-in
  // nonce hashed together with the participant, matching what a real QR
  // scan's jti-derived challenge would look like.
  const challengeHash = sha256(`${participant.address}:${checkpointPda}:${randomBytes(16).toString("hex")}`);
  record("6. Challenge generated", `challengeHash=${Buffer.from(challengeHash).toString("hex")}`);

  // --- Step 7 & 8: check in (participant + attestor co-sign), confirm ----
  const [attendancePda] = await findAttendancePda({ event: eventPda, checkpoint: checkpointPda, participant: participant.address });
  const checkInIx = await getCheckInInstructionAsync({
    participant,
    attestor,
    event: eventPda,
    checkpoint: checkpointPda,
    participantEvent: participantEventPda,
    challengeHash,
  });
  const checkInSignature = await signAndSend([checkInIx], participant);
  record("7-8. Check-in confirmed", `attendancePda=${attendancePda} tx=${checkInSignature}`);

  // --- Step 9: read and validate the Attendance PDA -----------------------
  const attendance = await fetchAttendanceAccount(rpc, attendancePda);
  const attendanceData = attendance.data;
  const eventMatches = attendanceData.event === eventPda;
  const checkpointMatches = attendanceData.checkpoint === checkpointPda;
  const participantMatches = attendanceData.participant === participant.address;
  const attestorMatches = attendanceData.attestor === attestor.address;
  const pointsMatch = attendanceData.pointsAwarded === points;
  if (!eventMatches || !checkpointMatches || !participantMatches || !attestorMatches || !pointsMatch) {
    throw new Error(
      `Attendance PDA fields don't match expectations: ${JSON.stringify({
        eventMatches,
        checkpointMatches,
        participantMatches,
        attestorMatches,
        pointsMatch,
        actual: attendanceData,
      })}`,
    );
  }
  record(
    "9. Attendance PDA validated",
    `event=${attendanceData.event} checkpoint=${attendanceData.checkpoint} participant=${attendanceData.participant} pointsAwarded=${attendanceData.pointsAwarded}`,
  );

  // --- Step 10 & 11: duplicate check-in must fail on-chain ----------------
  const duplicateChallengeHash = sha256(`${participant.address}:${checkpointPda}:${randomBytes(16).toString("hex")}`);
  let duplicateRejected = false;
  let duplicateErrorMessage = "";
  try {
    const duplicateCheckInIx = await getCheckInInstructionAsync({
      participant,
      attestor,
      event: eventPda,
      checkpoint: checkpointPda,
      participantEvent: participantEventPda,
      challengeHash: duplicateChallengeHash,
    });
    await signAndSend([duplicateCheckInIx], participant);
  } catch (error) {
    duplicateRejected = true;
    duplicateErrorMessage = error instanceof Error ? error.message : String(error);
  }
  if (!duplicateRejected) {
    throw new Error("duplicate check-in was NOT rejected on-chain — attendance PDA re-initialization succeeded, which must never happen");
  }
  record("10-11. Duplicate check-in rejected", `error=${duplicateErrorMessage.slice(0, 200)}`);

  // --- Step 12: produce a report (no secrets) -----------------------------
  console.log("\n=== devnet-smoke-test: PASSED ===");
  console.log(
    JSON.stringify(
      {
        network: "devnet",
        rpcHttpUrl,
        programId: EVENTQUEST_PROGRAM_ADDRESS,
        testedAt: new Date().toISOString(),
        wallets: { organizer: organizer.address, participant: participant.address, attestor: attestor.address },
        pdas: { event: eventPda, checkpoint: checkpointPda, participantEvent: participantEventPda, attendance: attendancePda },
        transactions: {
          fundWallets: { signature: fundingSignature, explorer: explorerTx(fundingSignature) },
          createEvent: { signature: createEventSignature, explorer: explorerTx(createEventSignature) },
          activateEvent: { signature: activateEventSignature, explorer: explorerTx(activateEventSignature) },
          createCheckpoint: { signature: createCheckpointSignature, explorer: explorerTx(createCheckpointSignature) },
          joinEvent: { signature: joinEventSignature, explorer: explorerTx(joinEventSignature) },
          checkIn: { signature: checkInSignature, explorer: explorerTx(checkInSignature) },
        },
        duplicateCheckIn: { rejected: true, errorMessage: duplicateErrorMessage },
        explorerAddresses: {
          event: explorerAddress(eventPda),
          checkpoint: explorerAddress(checkpointPda),
          attendance: explorerAddress(attendancePda),
        },
      },
      null,
      2,
    ),
  );
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error("\n=== devnet-smoke-test: FAILED ===");
    console.error(error instanceof Error ? (error.stack ?? error.message) : error);
    process.exit(1);
  });
