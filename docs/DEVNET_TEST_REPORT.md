# EventQuest — Devnet Test Report

Spec §0.8 evidence package. Every signature, PDA, and pubkey below is real
and independently verifiable on Solana Explorer / via RPC — nothing here
was produced by a mock, a local validator, or a simulated transaction.
This report documents two things:

1. The program's original Devnet deployment (Phase 1 provisioning).
2. A fresh, full run of `scripts/devnet-smoke-test.ts` (Phase 6), executed
   directly against that deployment on 2026-07-26.

## Deployment

| Field | Value |
|---|---|
| Cluster | `devnet` |
| Program ID | `CuULC7T6d49mf89sRPU8WLeYfkF74kDyEKvfCe2rXq8H` |
| Deployed at | `2026-07-25T13:44:00Z` |
| Deploy signature | [`2GU8WKq4XRg19U3V8iDEP6dB9jMbYmZDZTVKXQ5tKNjYvpMUTGB5KQ8hZQniDhH7NYVASrRR339bLukDYsN5hJPB`](https://explorer.solana.com/tx/2GU8WKq4XRg19U3V8iDEP6dB9jMbYmZDZTVKXQ5tKNjYvpMUTGB5KQ8hZQniDhH7NYVASrRR339bLukDYsN5hJPB?cluster=devnet) — re-confirmed `finalized` via `getSignatureStatuses` on 2026-07-26 |
| IDL SHA-256 | `a0afa6080718635488d3589a50ec992d521b48dcfc4513881987d8813c0b7b15` |
| Git commit at deploy | `77af3de70f9af9c8ed8aaf7b0dfc99a5374ae79c` |
| Explorer | <https://explorer.solana.com/address/CuULC7T6d49mf89sRPU8WLeYfkF74kDyEKvfCe2rXq8H?cluster=devnet> |

See `deployments/devnet.json` for the machine-readable record. `programs/`
has had no source changes since the deploy commit (`git diff --stat
77af3de..HEAD -- programs/` is empty and `77af3de` is an ancestor of the
commit this report was tested against), and the program account is
independently confirmed `executable: true` on Devnet as of this report's
test run — so the original deploy signature above remains the accurate
proof for the bytecode currently live at this Program ID; no redeploy was
needed for this test.

## Smoke test run (`pnpm devnet:smoke`, spec §0.9's 12-step flow)

| Field | Value |
|---|---|
| Tested at | `2026-07-26T04:25:27.609Z` |
| Commit tested | `80849f8f307c28b900f769d7278f90c7dde0cc7e` |
| RPC endpoint | `https://api.devnet.solana.com` |
| Exit code | `0` (script asserts every step and exits non-zero on any failure — verified separately: setting `SOLANA_NETWORK=mainnet` makes it exit `1` at step 1, before touching the chain) |

### Public test wallets

No private keys or seed phrases appear anywhere in this report or in
`scripts/devnet-smoke-test.ts`'s output. The organizer and participant
keypairs are generated fresh in-memory per run and discarded; the attestor
keypair (`.secrets/attestor-devnet.json`, gitignored) funds them with a
small direct SOL transfer instead of the public faucet, since the faucet
is aggressively rate-limited.

| Role | Public key |
|---|---|
| Organizer (event authority) | `Hjm1v6xmvY5PetxGfwa8Kg6JvgE9mB6Tm5RLrAs4mquG` |
| Participant | `5wM9pShjHDaeJs3yCyBvkYNsG97Kopi8D7HptMM4V9VQ` |
| Attestor (checkpoint co-signer) | `5JXydMjyYSpyQNCpW2L5hruLqUmkkbtU4fzUb8A2Nq6z` |

### PDAs created

| Account | Address | Explorer |
|---|---|---|
| Event | `DRorQUFfCAW3kbZghfVJtzKs51Qgd994NrLWDUjDxHPZ` | [link](https://explorer.solana.com/address/DRorQUFfCAW3kbZghfVJtzKs51Qgd994NrLWDUjDxHPZ?cluster=devnet) |
| Checkpoint | `5gdHXKx1TPBaABdCLmbztG6s675sQDqmr6JziPtZ9HGH` | [link](https://explorer.solana.com/address/5gdHXKx1TPBaABdCLmbztG6s675sQDqmr6JziPtZ9HGH?cluster=devnet) |
| ParticipantEvent | `CkHeSVnCWFRLZmcz4TfwQQZkGGbd5NqnyBmmWLEjVYc9` | — |
| Attendance | `GMwchPY3CiHqZ8YfAwZkBnpPnL6jSDDbwV9trKyrRLU3` | [link](https://explorer.solana.com/address/GMwchPY3CiHqZ8YfAwZkBnpPnL6jSDDbwV9trKyrRLU3?cluster=devnet) |

### Transactions (every one real, confirmed on Devnet)

| Step | Signature | Explorer |
|---|---|---|
| 3. Fund organizer + participant | `24x7Z4ngAZ93EdaV1MivDYZsVHWaTsMFxvaweBiudc3ScrkRam4sdCVkdRcSqqqvB8nL69aLfj6sx3fXGKfFo7Ze` | [link](https://explorer.solana.com/tx/24x7Z4ngAZ93EdaV1MivDYZsVHWaTsMFxvaweBiudc3ScrkRam4sdCVkdRcSqqqvB8nL69aLfj6sx3fXGKfFo7Ze?cluster=devnet) |
| 4. `initialize_event` | `S9jZMAnmLWbYcPLw1fiKMHeH5kCgn5ZjYzenarPnSxM2maE2gsoFCtzsu5vyH6qLoMQ7LHQgt74gcVM1gJuE3EY` | [link](https://explorer.solana.com/tx/S9jZMAnmLWbYcPLw1fiKMHeH5kCgn5ZjYzenarPnSxM2maE2gsoFCtzsu5vyH6qLoMQ7LHQgt74gcVM1gJuE3EY?cluster=devnet) |
| 4b. `update_event_status` → Active | `kf2X3Kqgne7HS9zyMJ7KiwWH7Ve5a2fRZuoGxoqD4WCpH1YxBqaFmspfjh293CmsFDuixBRRPdXMSnAw92Kw6YR` | [link](https://explorer.solana.com/tx/kf2X3Kqgne7HS9zyMJ7KiwWH7Ve5a2fRZuoGxoqD4WCpH1YxBqaFmspfjh293CmsFDuixBRRPdXMSnAw92Kw6YR?cluster=devnet) |
| 5. `create_checkpoint` (points=10) | `5T1oYYBU16fLmLVwuyqHUwk7UkjDuea3QCf6GcvF9J5tzHUcyzCqsXBGhyzbTfBXYXb2hjCUvp9H9FcUZYimJTW7` | [link](https://explorer.solana.com/tx/5T1oYYBU16fLmLVwuyqHUwk7UkjDuea3QCf6GcvF9J5tzHUcyzCqsXBGhyzbTfBXYXb2hjCUvp9H9FcUZYimJTW7?cluster=devnet) |
| 5b. `join_event` | `5byNEz7LKt5m7oTiQWuPJGgcKe6j8UCT11Ls1TWs2iPsDZKbxANJYTHn5tYLWCyASDatKnuV8DJowmNBPVTqzsbk` | [link](https://explorer.solana.com/tx/5byNEz7LKt5m7oTiQWuPJGgcKe6j8UCT11Ls1TWs2iPsDZKbxANJYTHn5tYLWCyASDatKnuV8DJowmNBPVTqzsbk?cluster=devnet) |
| 7-8. `check_in` (participant + attestor co-signed) | `4fFbfYdraxXAFrQUMvSgxqWrXLyjqLEDhM1AsaWHEu4HboESk5tu4ofzKofZVBurFokfCmveXQyUhi68hF7x44oC` | [link](https://explorer.solana.com/tx/4fFbfYdraxXAFrQUMvSgxqWrXLyjqLEDhM1AsaWHEu4HboESk5tu4ofzKofZVBurFokfCmveXQyUhi68hF7x44oC?cluster=devnet) |

### Step 9 — Attendance PDA validation

Fetched and decoded directly from the account returned by `check_in`:

- `event` = `DRorQUFfCAW3kbZghfVJtzKs51Qgd994NrLWDUjDxHPZ` ✓ matches
- `checkpoint` = `5gdHXKx1TPBaABdCLmbztG6s675sQDqmr6JziPtZ9HGH` ✓ matches
- `participant` = `5wM9pShjHDaeJs3yCyBvkYNsG97Kopi8D7HptMM4V9VQ` ✓ matches
- `attestor` = `5JXydMjyYSpyQNCpW2L5hruLqUmkkbtU4fzUb8A2Nq6z` ✓ matches
- `pointsAwarded` = `10` ✓ matches the checkpoint's configured points (never trusted from a client — spec §11.5)

### Steps 10-11 — Duplicate check-in rejection

A second `check_in` for the same (event, checkpoint, participant) tuple —
with a freshly generated challenge hash, everything else identical — was
submitted and **rejected on-chain**: `Transaction simulation failed`,
because the `attendance` PDA's `init` constraint fails when the account
already exists (seeds are unique per (event, checkpoint, participant) —
see `programs/eventquest/src/instructions/check_in.rs`). No new Attendance
account was created; `pointsAwarded` was not double-counted.

### Step 6 — Challenge generation note

The on-chain program only requires `challengeHash` to be non-zero (see
`check_in_handler`); it never inspects the value itself — QR/JWT
validation is entirely an off-chain (`apps/api`) concern. The smoke test
generates a random per-check-in 32-byte hash to exercise the same
data shape a real QR-derived challenge would have.

## Full-stack browser E2E on Devnet (spec §25.6)

`scripts/devnet-e2e.sh` runs `apps/web/tests/e2e-devnet/devnet-vertical-scenario.spec.ts`
— the same vertical scenario as Phase 5's local-Surfnet suite, but with
every service pointed at real Devnet instead: a production `apps/web`
build, a real `apps/api` and `apps/indexer` against real Postgres/Redis,
the real deployed program above, and a real Playwright-driven browser
wallet (`apps/web/tests/e2e/inject-test-wallet.js`) funded by a direct
Devnet transfer from the attestor keypair (see "known limitations" below).

| Field | Value |
|---|---|
| Run at | `2026-07-26T04:37:xx UTC` (event/checkpoint rows created `2026-07-26 04:37:46`–`04:37:48`) |
| Result | `1 passed`, exit code `0` |
| Event created | `5aa77b8f-738c-4bf3-bacf-21bec35cb90f` ("Devnet E2E Event …") |
| Checkpoint created | `5fd20cce-5e84-4746-91b6-d15c0a2a2af0` ("Devnet E2E Checkpoint …") |
| Check-in transaction | [`CfKjKsm13DWYWYAciZXyqYjeGwbVmMf8ajoVxH3PFn8kp7CDvcnDJpomzcfdBzSsard5NpNnyFYCuMNX4XMx3e4`](https://explorer.solana.com/tx/CfKjKsm13DWYWYAciZXyqYjeGwbVmMf8ajoVxH3PFn8kp7CDvcnDJpomzcfdBzSsard5NpNnyFYCuMNX4XMx3e4?cluster=devnet) — independently re-confirmed `finalized` via `getSignatureStatuses` after the test run |
| Points awarded | `10` (read from Postgres `attendances.points_awarded`, sourced from the indexer's decode of the real `AttendanceRecorded` event) |
| Duplicate check-in | Rejected — the participant page showed the error alert and never reached `/check-in/result` for the second scan |

This proves the full pipeline for real: browser wallet signs → `apps/api`
prepares/submits → Devnet executes the program → `apps/indexer` picks up
the real `AttendanceRecorded` event → Postgres is updated → the organizer's
dashboard reflects it, all without a mock or local validator anywhere in
the loop.

## Versions

| Tool | Version |
|---|---|
| Solana CLI | `solana-cli 3.1.10 (src:7bc9c805; feat:1620780344, client:Agave)` |
| Anchor CLI | `1.1.2` |
| Rust | `1.97.1 (8bab26f4f 2026-07-14)` |
| Node | `v24.16.0` |
| pnpm | `9.15.0` |

## Known limitations

- **Faucet not exercised.** Both the smoke test and the full-stack E2E
  fund fresh wallets via a direct transfer from the already-funded
  attestor keypair rather than the public Devnet faucet, which is
  rate-limited enough to make repeat automated runs unreliable. The
  faucet path itself (a real wallet requesting SOL directly) is exercised
  manually and by `apps/web`'s own UI, not by these scripts.
- **Program bytecode unchanged since Phase 1.** This report's tests prove
  the deployed program still behaves correctly; they do not represent a
  new deploy. `deployments/devnet.json`'s `deploySignature` is the
  original and only deploy transaction for this Program ID.
- **`apps/api`'s own `#[ignore]`d Devnet integration tests**
  (`checkins::devnet_tests`, `organizer_devnet_tests`, `apps/indexer`'s
  `devnet_tests`) separately prove the same backend-mediated check-in flow
  at the Rust level against this same program on Devnet, run via
  `cargo test -- --ignored`.
- **Supporting scripts not yet committed** at the time this report was
  generated (`scripts/devnet-smoke-test.ts`, `scripts/verify-devnet-config.sh`,
  `scripts/devnet-e2e.sh`, `apps/web/tests/e2e-devnet/`, this file, and the
  new package dependencies) — the on-chain proof above is unaffected (no
  program changes), but `git commit` is pending.
