//! Real end-to-end proof against the deployed Devnet program and a live
//! Postgres/Redis: provisions an event + checkpoint + a confirmed check-in
//! entirely on-chain (mirroring `apps/api/src/checkins/devnet_tests.rs`'s
//! own simplification — the attestor stands in for the organizer), mirrors
//! the off-chain rows a real `apps/api` deployment would already have,
//! then runs this crate's own `process::run_pass` against real Devnet and
//! real Postgres/Redis and confirms the resulting
//! `attendances`/`event_participants`/`checkin_attempts` rows — proving
//! the indexer decodes a genuine `AttendanceRecorded` log and persists
//! idempotently on a second pass.
//!
//! Ignored by default — `cargo test -p eventquest-indexer -- --ignored`.
//! Requires: `docker-compose up` (Postgres + Redis), a Devnet RPC, and
//! `ATTESTOR_KEYPAIR_PATH` funded with Devnet SOL.

use std::time::Duration;

use anchor_lang::prelude::Pubkey;
use anchor_lang::solana_program::instruction::Instruction;
use anchor_lang::solana_program::system_instruction;
use anchor_lang::{InstructionData, ToAccountMetas};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use chrono::Utc;
use ed25519_dalek::{Signer as DalekSigner, SigningKey};
use eventquest_chain::client::{accounts, args};
use eventquest_chain::{pda, types::EventStatus, ID as PROGRAM_ID};
use serde_json::json;
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

use crate::process::{self, Cursor};
use crate::rpc::RpcClient;

fn env_or(name: &str, default: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| default.to_string())
}

/// A tiny write-capable RPC helper for test setup only — the production
/// `rpc::RpcClient` deliberately has no `sendTransaction`/blockhash
/// methods, since the indexer itself never submits anything.
struct TestChainWriter {
    http: reqwest::Client,
    url: String,
}

impl TestChainWriter {
    fn new(url: String) -> Self {
        Self {
            http: reqwest::Client::new(),
            url,
        }
    }

    async fn call(&self, method: &str, params: serde_json::Value) -> serde_json::Value {
        let body = json!({ "jsonrpc": "2.0", "id": 1, "method": method, "params": params });
        let response = self
            .http
            .post(&self.url)
            .json(&body)
            .send()
            .await
            .expect("RPC request failed");
        let parsed: serde_json::Value = response.json().await.expect("RPC response should parse");
        if let Some(error) = parsed.get("error") {
            panic!("RPC {method} failed: {error}");
        }
        parsed["result"].clone()
    }

    async fn latest_blockhash(&self) -> solana_hash::Hash {
        let result = self
            .call("getLatestBlockhash", json!([{ "commitment": "confirmed" }]))
            .await;
        result["value"]["blockhash"]
            .as_str()
            .expect("blockhash string")
            .parse()
            .expect("valid blockhash")
    }

    async fn send_and_confirm(&self, tx_base64: &str) -> String {
        let signature = self
            .call(
                "sendTransaction",
                json!([
                    tx_base64,
                    { "encoding": "base64", "skipPreflight": false, "preflightCommitment": "confirmed" }
                ]),
            )
            .await
            .as_str()
            .expect("signature string")
            .to_string();

        let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
        loop {
            let result = self
                .call(
                    "getSignatureStatuses",
                    json!([[&signature], { "searchTransactionHistory": true }]),
                )
                .await;
            let status = &result["value"][0];
            if !status.is_null() {
                let confirmation = status["confirmationStatus"].as_str().unwrap_or("");
                if confirmation == "confirmed" || confirmation == "finalized" {
                    return signature;
                }
            }
            if tokio::time::Instant::now() >= deadline {
                panic!("transaction {signature} did not confirm within 30s");
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }

    async fn submit_signed_by(&self, signer: &SigningKey, instructions: &[Instruction]) -> String {
        let fee_payer = Pubkey::new_from_array(signer.verifying_key().to_bytes());
        let blockhash = self.latest_blockhash().await;
        let message =
            solana_message::Message::new_with_blockhash(instructions, Some(&fee_payer), &blockhash);
        let mut transaction = solana_transaction::Transaction::new_unsigned(message);
        let message_bytes = transaction.message_data();
        let signature_bytes = signer.sign(&message_bytes).to_bytes();
        transaction.signatures[0] = solana_signature::Signature::from(signature_bytes);
        let serialized = bincode::serialize(&transaction).expect("transaction always serializes");
        self.send_and_confirm(&BASE64.encode(serialized)).await
    }
}

async fn load_attestor(path: &str) -> SigningKey {
    let bytes = tokio::fs::read(path)
        .await
        .expect("failed to read attestor keypair");
    let keypair: Vec<u8> = serde_json::from_slice(&bytes).expect("valid keypair JSON");
    let seed: [u8; 32] = keypair[0..32].try_into().expect("32-byte seed");
    SigningKey::from_bytes(&seed)
}

#[tokio::test]
#[ignore = "hits live Solana Devnet + local Postgres/Redis; requires docker-compose up and a funded ATTESTOR_KEYPAIR_PATH"]
async fn indexes_a_real_attendance_recorded_event_idempotently() {
    let database_url = env_or(
        "DATABASE_URL",
        "postgresql://eventquest:eventquest@localhost:5432/eventquest",
    );
    let redis_url = env_or("REDIS_URL", "redis://localhost:6379");
    let rpc_http_url = env_or("SOLANA_RPC_HTTP_URL", "https://api.devnet.solana.com");
    let attestor_keypair_path = env_or("ATTESTOR_KEYPAIR_PATH", ".secrets/attestor-devnet.json");

    let db = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("failed to connect to Postgres — is `docker-compose up` running?");
    let redis_client = redis::Client::open(redis_url).expect("invalid REDIS_URL");
    let mut redis = redis::aio::ConnectionManager::new(redis_client)
        .await
        .expect("failed to connect to Redis — is `docker-compose up` running?");

    let attestor = load_attestor(&attestor_keypair_path).await;
    let attestor_pubkey = Pubkey::new_from_array(attestor.verifying_key().to_bytes());
    let chain = TestChainWriter::new(rpc_http_url.clone());
    let rpc = RpcClient::new(rpc_http_url);

    // --- Provision event + checkpoint + a confirmed check-in on-chain ----
    let external_id_hash: [u8; 32] = rand::random();
    let (event_pda, _) = pda::event(&attestor_pubkey, &external_id_hash);
    let now_ts = Utc::now().timestamp();

    chain
        .submit_signed_by(
            &attestor,
            &[Instruction {
                program_id: PROGRAM_ID,
                accounts: accounts::InitializeEvent {
                    authority: attestor_pubkey,
                    event: event_pda,
                    system_program: anchor_lang::system_program::ID,
                }
                .to_account_metas(None),
                data: args::InitializeEvent {
                    external_id_hash,
                    starts_at: now_ts - 3600,
                    ends_at: now_ts + 3600,
                }
                .data(),
            }],
        )
        .await;

    chain
        .submit_signed_by(
            &attestor,
            &[Instruction {
                program_id: PROGRAM_ID,
                accounts: accounts::UpdateEventStatus {
                    authority: attestor_pubkey,
                    event: event_pda,
                }
                .to_account_metas(None),
                data: args::UpdateEventStatus {
                    new_status: EventStatus::Active,
                }
                .data(),
            }],
        )
        .await;

    let checkpoint_hash: [u8; 32] = rand::random();
    let (checkpoint_pda, _) = pda::checkpoint(&event_pda, &checkpoint_hash);
    chain
        .submit_signed_by(
            &attestor,
            &[Instruction {
                program_id: PROGRAM_ID,
                accounts: accounts::CreateCheckpoint {
                    authority: attestor_pubkey,
                    event: event_pda,
                    checkpoint: checkpoint_pda,
                    system_program: anchor_lang::system_program::ID,
                }
                .to_account_metas(None),
                data: args::CreateCheckpoint {
                    external_id_hash: checkpoint_hash,
                    attestor: attestor_pubkey,
                    opens_at: now_ts - 3600,
                    closes_at: now_ts + 3600,
                    points: 75,
                }
                .data(),
            }],
        )
        .await;

    let participant_seed: [u8; 32] = rand::random();
    let participant = SigningKey::from_bytes(&participant_seed);
    let participant_pubkey = Pubkey::new_from_array(participant.verifying_key().to_bytes());
    chain
        .submit_signed_by(
            &attestor,
            &[system_instruction::transfer(
                &attestor_pubkey,
                &participant_pubkey,
                10_000_000,
            )],
        )
        .await;

    let (participant_event_pda, _) = pda::participant_event(&event_pda, &participant_pubkey);
    chain
        .submit_signed_by(
            &participant,
            &[Instruction {
                program_id: PROGRAM_ID,
                accounts: accounts::JoinEvent {
                    participant: participant_pubkey,
                    event: event_pda,
                    participant_event: participant_event_pda,
                    system_program: anchor_lang::system_program::ID,
                }
                .to_account_metas(None),
                data: args::JoinEvent {}.data(),
            }],
        )
        .await;

    let (attendance_pda, _) = pda::attendance(&event_pda, &checkpoint_pda, &participant_pubkey);
    let challenge_hash: [u8; 32] = rand::random();

    // check_in needs both participant and attestor signatures — build it
    // manually here (this test holds both keys locally), unlike apps/api's
    // split build-then-co-sign-then-participant-signs flow.
    let check_in_ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: accounts::CheckIn {
            participant: participant_pubkey,
            attestor: attestor_pubkey,
            event: event_pda,
            checkpoint: checkpoint_pda,
            participant_event: participant_event_pda,
            attendance: attendance_pda,
            system_program: anchor_lang::system_program::ID,
        }
        .to_account_metas(None),
        data: args::CheckIn { challenge_hash }.data(),
    };
    let blockhash = chain.latest_blockhash().await;
    let message = solana_message::Message::new_with_blockhash(
        &[check_in_ix],
        Some(&participant_pubkey),
        &blockhash,
    );
    let mut transaction = solana_transaction::Transaction::new_unsigned(message);
    let message_bytes = transaction.message_data();
    let participant_index = transaction
        .message
        .signer_keys()
        .iter()
        .position(|&&key| key == participant_pubkey)
        .expect("participant should be a required signer");
    let attestor_index = transaction
        .message
        .signer_keys()
        .iter()
        .position(|&&key| key == attestor_pubkey)
        .expect("attestor should be a required signer");
    transaction.signatures[participant_index] =
        solana_signature::Signature::from(participant.sign(&message_bytes).to_bytes());
    transaction.signatures[attestor_index] =
        solana_signature::Signature::from(attestor.sign(&message_bytes).to_bytes());
    let serialized = bincode::serialize(&transaction).expect("transaction always serializes");
    let check_in_signature = chain.send_and_confirm(&BASE64.encode(serialized)).await;
    println!(
        "check_in signature: https://explorer.solana.com/tx/{check_in_signature}?cluster=devnet"
    );

    // --- Mirror the off-chain rows a real apps/api deployment would ------
    // already have created (see apps/api/src/checkins/mod.rs's header
    // comment on the organizer on-chain-provisioning gap this stands in
    // for).
    let organization_id: Uuid =
        sqlx::query_scalar("insert into organizations (name, slug) values ($1, $2) returning id")
            .bind("Indexer Test Org")
            .bind(format!("indexer-test-org-{}", Uuid::new_v4()))
            .fetch_one(&db)
            .await
            .expect("failed to insert test organization");

    let starts_at = Utc::now() - chrono::Duration::hours(1);
    let ends_at = Utc::now() + chrono::Duration::hours(1);
    let event_id: Uuid = sqlx::query_scalar(
        "insert into events ( \
             organization_id, name, slug, timezone, starts_at, ends_at, \
             status, visibility, solana_network, onchain_event_address, created_by_wallet \
         ) values ($1, 'Indexer Test Event', $2, 'UTC', $3, $4, 'active', 'private', 'devnet', $5, $6) \
         returning id",
    )
    .bind(organization_id)
    .bind(format!("indexer-test-event-{}", Uuid::new_v4()))
    .bind(starts_at)
    .bind(ends_at)
    .bind(event_pda.to_string())
    .bind(attestor_pubkey.to_string())
    .fetch_one(&db)
    .await
    .expect("failed to insert test event");

    let checkpoint_id: Uuid = sqlx::query_scalar(
        "insert into checkpoints ( \
             event_id, name, points, rotation_seconds, opens_at, closes_at, \
             status, attestor_pubkey, onchain_checkpoint_address \
         ) values ($1, 'Indexer Test Checkpoint', 75, 15, $2, $3, 'active', $4, $5) \
         returning id",
    )
    .bind(event_id)
    .bind(starts_at)
    .bind(ends_at)
    .bind(attestor_pubkey.to_string())
    .bind(checkpoint_pda.to_string())
    .fetch_one(&db)
    .await
    .expect("failed to insert test checkpoint");

    let participant_id: Uuid =
        sqlx::query_scalar("insert into participants (wallet_address) values ($1) returning id")
            .bind(participant_pubkey.to_string())
            .fetch_one(&db)
            .await
            .expect("failed to insert test participant");

    sqlx::query("insert into event_participants (event_id, participant_id) values ($1, $2)")
        .bind(event_id)
        .bind(participant_id)
        .execute(&db)
        .await
        .expect("failed to insert event_participants row");

    let grant_id = Uuid::new_v4();
    sqlx::query(
        "insert into checkin_attempts ( \
             grant_id, event_id, checkpoint_id, participant_id, qr_jti_hash, \
             challenge_hash, idempotency_key, status, transaction_signature, expires_at \
         ) values ($1, $2, $3, $4, 'test-jti-hash', $5, $6, 'transaction_submitted', $7, now() + interval '30 seconds')",
    )
    .bind(grant_id)
    .bind(event_id)
    .bind(checkpoint_id)
    .bind(participant_id)
    .bind(hex::encode(challenge_hash))
    .bind(grant_id.to_string())
    .bind(&check_in_signature)
    .execute(&db)
    .await
    .expect("failed to insert test checkin_attempts row");

    // --- Run the real indexer pass against real Devnet + real Postgres --
    let cursor = Cursor {
        network: "devnet".to_string(),
        program_id: PROGRAM_ID.to_string(),
    };
    let processed = process::run_pass(&db, &mut redis, &rpc, &cursor, 200, "confirmed")
        .await
        .expect("indexer pass should succeed");
    assert!(
        processed >= 1,
        "the indexer should have processed at least this test's own check_in transaction"
    );

    let attendance_row: (Uuid, Uuid, Uuid, i32, String) = sqlx::query_as(
        "select event_id, checkpoint_id, participant_id, points_awarded, transaction_signature \
         from attendances where transaction_signature = $1",
    )
    .bind(&check_in_signature)
    .fetch_one(&db)
    .await
    .expect("attendance row should exist after indexing");
    assert_eq!(attendance_row.0, event_id);
    assert_eq!(attendance_row.1, checkpoint_id);
    assert_eq!(attendance_row.2, participant_id);
    assert_eq!(attendance_row.3, 75);

    let (points, checkin_count): (i64, i32) = sqlx::query_as(
        "select points, checkin_count from event_participants where event_id = $1 and participant_id = $2",
    )
    .bind(event_id)
    .bind(participant_id)
    .fetch_one(&db)
    .await
    .expect("event_participants row should exist");
    assert_eq!(points, 75);
    assert_eq!(checkin_count, 1);

    let attempt_status: String =
        sqlx::query_scalar("select status::text from checkin_attempts where grant_id = $1")
            .bind(grant_id)
            .fetch_one(&db)
            .await
            .expect("checkin_attempts row should exist");
    assert_eq!(attempt_status, "confirmed");

    // --- Idempotency: a second pass must not double-count ---------------
    process::run_pass(&db, &mut redis, &rpc, &cursor, 200, "confirmed")
        .await
        .expect("second indexer pass should succeed");

    let attendance_count: i64 =
        sqlx::query_scalar("select count(*) from attendances where transaction_signature = $1")
            .bind(&check_in_signature)
            .fetch_one(&db)
            .await
            .unwrap();
    assert_eq!(
        attendance_count, 1,
        "reprocessing the same signature must not create a duplicate row"
    );

    let (points_after, checkin_count_after): (i64, i32) = sqlx::query_as(
        "select points, checkin_count from event_participants where event_id = $1 and participant_id = $2",
    )
    .bind(event_id)
    .bind(participant_id)
    .fetch_one(&db)
    .await
    .unwrap();
    assert_eq!(
        points_after, 75,
        "reprocessing must not double-award points"
    );
    assert_eq!(checkin_count_after, 1);

    println!("attendance PDA: https://explorer.solana.com/address/{attendance_pda}?cluster=devnet");
}
