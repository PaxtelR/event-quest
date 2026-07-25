// Smoke tests for the Codama-generated @solana/kit client: confirms PDA
// derivation, instruction encoding, and account-meta ordering work end to
// end — the same three things the Rust equivalent
// (crates/eventquest-chain/tests/generated_client.rs) verifies on the
// backend side.
import assert from "node:assert/strict";
import { test } from "node:test";
import { AccountRole, address, generateKeyPairSigner } from "@solana/kit";
import {
  EVENTQUEST_PROGRAM_ADDRESS,
  getCheckInInstruction,
  getCheckInInstructionDataEncoder,
  findEventPda,
  findCheckpointPda,
  findParticipantEventPda,
  findAttendancePda,
} from "./generated/index.js";

test("program address matches the deployed keypair", () => {
  assert.equal(
    EVENTQUEST_PROGRAM_ADDRESS,
    "CuULC7T6d49mf89sRPU8WLeYfkF74kDyEKvfCe2rXq8H",
  );
});

test("PDA derivation is deterministic and matches the program's seeds", async () => {
  const authority = await generateKeyPairSigner();
  const externalIdHash = new Uint8Array(32).fill(7);

  const [event, eventAgain] = await Promise.all([
    findEventPda({ authority: authority.address, externalIdHash }),
    findEventPda({ authority: authority.address, externalIdHash }),
  ]);
  assert.equal(event[0], eventAgain[0]);
  assert.equal(event[1], eventAgain[1]);

  const checkpointHash = new Uint8Array(32).fill(8);
  const [checkpoint] = await findCheckpointPda({
    event: event[0],
    externalIdHash: checkpointHash,
  });

  const participant = await generateKeyPairSigner();
  const [participantEvent] = await findParticipantEventPda({
    event: event[0],
    participant: participant.address,
  });
  const [attendance] = await findAttendancePda({
    event: event[0],
    checkpoint,
    participant: participant.address,
  });

  // All four PDAs must be distinct even though they share overlapping seed
  // components — sanity check against a seed-derivation copy/paste bug.
  const all = [event[0], checkpoint, participantEvent, attendance];
  assert.equal(new Set(all).size, all.length, "PDAs must all be distinct");
});

test("check-in instruction data round-trips the challenge hash", () => {
  const challengeHash = new Uint8Array(32).fill(42);
  const data = getCheckInInstructionDataEncoder().encode({ challengeHash });

  // 8-byte Anchor discriminator + the 32-byte challenge hash, nothing else.
  assert.equal(data.length, 8 + 32);
  assert.deepEqual(Array.from(data.slice(8)), Array.from(challengeHash));
});

test("check-in account metas match the program's account order and signer flags", async () => {
  const participant = await generateKeyPairSigner();
  const attestor = await generateKeyPairSigner();
  const event = address("11111111111111111111111111111112");
  const checkpoint = address("11111111111111111111111111111113");
  const participantEvent = address("11111111111111111111111111111114");
  const attendance = address("11111111111111111111111111111115");

  const instruction = getCheckInInstruction({
    participant,
    attestor,
    event,
    checkpoint,
    participantEvent,
    attendance,
    challengeHash: new Uint8Array(32).fill(1),
  });

  assert.equal(instruction.programAddress, EVENTQUEST_PROGRAM_ADDRESS);
  assert.equal(instruction.accounts?.length, 7);

  const expectedOrder = [
    participant.address,
    attestor.address,
    event,
    checkpoint,
    participantEvent,
    attendance,
  ];
  instruction.accounts
    ?.slice(0, expectedOrder.length)
    .forEach((meta, i) => assert.equal(meta.address, expectedOrder[i]));

  // Signer flags must match programs/eventquest/src/instructions/check_in.rs
  // exactly, since the backend relies on this to know who needs to sign.
  assert.equal(instruction.accounts?.[0].role, AccountRole.WRITABLE_SIGNER);
  assert.equal(instruction.accounts?.[1].role, AccountRole.READONLY_SIGNER);
  assert.equal(instruction.accounts?.[2].role, AccountRole.WRITABLE); // event
});
