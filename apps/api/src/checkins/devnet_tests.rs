//! Real end-to-end proof against the deployed Devnet program and a live
//! Postgres/Redis (`docker-compose up`, spec §0.9's smoke-test spirit
//! applied to this module specifically): provisions an event + checkpoint
//! on-chain (the attestor stands in for the organizer's wallet, same
//! simplification `blockchain::client::devnet_tests` already uses), mints a
//! real rotating QR token via the production `qr` module, then drives the
//! actual `validate` → `prepare-transaction` → `submitted` handlers, has a
//! freshly generated participant keypair sign and submit the returned
//! transaction, and confirms the resulting Attendance/ParticipantEvent PDAs
//! on-chain. Finally proves the on-chain duplicate-check-in guard by
//! resubmitting the same `check_in` instruction and asserting Devnet
//! rejects it.
//!
//! Ignored by default — `cargo test -p eventquest-api -- --ignored
//! checkins::devnet_tests`. Requires: `docker-compose up` (Postgres +
//! Redis), a Devnet RPC, and `ATTESTOR_KEYPAIR_PATH` funded with Devnet SOL.

use std::time::Duration;

use anchor_lang::prelude::Pubkey;
use anchor_lang::solana_program::instruction::Instruction;
use anchor_lang::solana_program::system_instruction;
use anchor_lang::{InstructionData, ToAccountMetas};
use axum::extract::{Path, State};
use axum::Json;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use chrono::Utc;
use ed25519_dalek::{Signer as DalekSigner, SigningKey};
use eventquest_chain::client::{accounts, args};
use eventquest_chain::{pda, types::EventStatus, ID as PROGRAM_ID};
use eventquest_config::{AppConfig, QrConfig, SessionConfig, SolanaConfig};
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

use crate::auth::AuthenticatedWallet;
use crate::blockchain::BlockchainClient;
use crate::error::ApiError;
use crate::qr;
use crate::state::AppState;

use super::handlers;
use super::models::{SubmittedRequest, ValidateRequest};

fn env_or(name: &str, default: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| default.to_string())
}

async fn build_test_state() -> AppState {
    let database_url = env_or(
        "DATABASE_URL",
        "postgresql://eventquest:eventquest@localhost:5432/eventquest",
    );
    let redis_url = env_or("REDIS_URL", "redis://localhost:6379");
    let rpc_http_url = env_or("SOLANA_RPC_HTTP_URL", "https://api.devnet.solana.com");
    let rpc_ws_url = env_or("SOLANA_RPC_WS_URL", "wss://api.devnet.solana.com");
    let attestor_keypair_path = env_or("ATTESTOR_KEYPAIR_PATH", ".secrets/attestor-devnet.json");

    let db = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("failed to connect to Postgres — is `docker-compose up` running?");
    sqlx::migrate!("./migrations")
        .run(&db)
        .await
        .expect("failed to apply migrations");

    let redis_client = redis::Client::open(redis_url.clone()).expect("invalid REDIS_URL");
    let redis = redis::aio::ConnectionManager::new(redis_client)
        .await
        .expect("failed to connect to Redis — is `docker-compose up` running?");

    let attestor_pubkey = crate::state::load_attestor_pubkey(&attestor_keypair_path)
        .await
        .expect("failed to load attestor keypair");

    let blockchain = BlockchainClient::new(&rpc_http_url, &attestor_keypair_path, PROGRAM_ID)
        .await
        .expect("failed to build BlockchainClient — is the attestor keypair funded?");

    let config = AppConfig {
        node_env: "test".to_string(),
        public_app_url: "http://localhost:3000".to_string(),
        api_url: "http://localhost:3001".to_string(),
        database_url,
        redis_url,
        solana: SolanaConfig {
            network: "devnet".to_string(),
            rpc_http_url,
            rpc_ws_url,
            program_id: PROGRAM_ID,
            attestor_keypair_path,
        },
        qr: QrConfig {
            rotation_seconds: 15,
            grace_seconds: 5,
            checkin_grant_seconds: 30,
            signing_secret: "test-qr-secret".to_string(),
        },
        session: SessionConfig {
            cookie_name: "eventquest_session".to_string(),
            ttl_seconds: 86_400,
            auth_signing_secret: "test-auth-secret".to_string(),
        },
        log_level: "info".to_string(),
        otel_exporter_otlp_endpoint: None,
    };

    AppState::new(config, db, redis, attestor_pubkey, blockchain)
}

/// The participant signs at signature index 0 — `build_and_co_sign` always
/// puts the fee payer (the participant, in every call this test makes)
/// there.
async fn participant_sign_and_submit(
    state: &AppState,
    participant: &SigningKey,
    tx_base64: &str,
) -> Result<String, ApiError> {
    let tx_bytes = BASE64.decode(tx_base64).expect("valid base64");
    let mut transaction: solana_transaction::Transaction =
        bincode::deserialize(&tx_bytes).expect("valid transaction bytes");
    let message_bytes = transaction.message_data();
    let signature_bytes = participant.sign(&message_bytes).to_bytes();
    transaction.signatures[0] = solana_signature::Signature::from(signature_bytes);
    let signed_bytes = bincode::serialize(&transaction).expect("transaction always serializes");
    state.blockchain.submit(&BASE64.encode(signed_bytes)).await
}

#[tokio::test]
#[ignore = "hits live Solana Devnet + local Postgres/Redis; requires docker-compose up and a funded ATTESTOR_KEYPAIR_PATH"]
async fn full_check_in_flow_on_devnet() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("eventquest_api=debug")
        .try_init();
    let state = build_test_state().await;
    let attestor_pubkey = state.blockchain.attestor_pubkey;

    // --- Provision event + checkpoint on-chain, attestor as organizer ----
    let external_id_hash: [u8; 32] = rand::random();
    let (event_pda, _) = pda::event(&attestor_pubkey, &external_id_hash);
    let now_ts = Utc::now().timestamp();

    let init_event_ix = Instruction {
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
    };
    let sig = state
        .blockchain
        .send_signed_by_attestor(&[init_event_ix])
        .await
        .expect("initialize_event failed");
    assert!(state
        .blockchain
        .confirm(&sig, Duration::from_secs(30))
        .await
        .expect("RPC error confirming initialize_event"));

    let update_status_ix = Instruction {
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
    };
    let sig = state
        .blockchain
        .send_signed_by_attestor(&[update_status_ix])
        .await
        .expect("update_event_status failed");
    assert!(state
        .blockchain
        .confirm(&sig, Duration::from_secs(30))
        .await
        .expect("RPC error confirming update_event_status"));

    let checkpoint_hash: [u8; 32] = rand::random();
    let (checkpoint_pda, _) = pda::checkpoint(&event_pda, &checkpoint_hash);
    let create_checkpoint_ix = Instruction {
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
            points: 50,
        }
        .data(),
    };
    let sig = state
        .blockchain
        .send_signed_by_attestor(&[create_checkpoint_ix])
        .await
        .expect("create_checkpoint failed");
    assert!(state
        .blockchain
        .confirm(&sig, Duration::from_secs(30))
        .await
        .expect("RPC error confirming create_checkpoint"));

    println!("event PDA: https://explorer.solana.com/address/{event_pda}?cluster=devnet");
    println!("checkpoint PDA: https://explorer.solana.com/address/{checkpoint_pda}?cluster=devnet");

    // --- Mirror these into the off-chain DB, as if an organizer-facing
    // on-chain-provisioning flow had already run (out of scope for this
    // module — see apps/api/src/checkins/mod.rs's header comment) --------
    let organization_id: Uuid =
        sqlx::query_scalar("insert into organizations (name, slug) values ($1, $2) returning id")
            .bind("Devnet Test Org")
            .bind(format!("devnet-test-org-{}", Uuid::new_v4()))
            .fetch_one(&state.db)
            .await
            .expect("failed to insert test organization");

    let starts_at = Utc::now() - chrono::Duration::hours(1);
    let ends_at = Utc::now() + chrono::Duration::hours(1);

    let event_id: Uuid = sqlx::query_scalar(
        "insert into events ( \
             organization_id, name, slug, timezone, starts_at, ends_at, \
             status, visibility, solana_network, onchain_event_address, created_by_wallet \
         ) values ($1, 'Devnet Test Event', $2, 'UTC', $3, $4, 'active', 'private', 'devnet', $5, $6) \
         returning id",
    )
    .bind(organization_id)
    .bind(format!("devnet-test-event-{}", Uuid::new_v4()))
    .bind(starts_at)
    .bind(ends_at)
    .bind(event_pda.to_string())
    .bind(state.attestor_pubkey.clone())
    .fetch_one(&state.db)
    .await
    .expect("failed to insert test event");

    let checkpoint_id: Uuid = sqlx::query_scalar(
        "insert into checkpoints ( \
             event_id, name, points, rotation_seconds, opens_at, closes_at, \
             status, attestor_pubkey, onchain_checkpoint_address \
         ) values ($1, 'Devnet Test Checkpoint', 50, 15, $2, $3, 'active', $4, $5) \
         returning id",
    )
    .bind(event_id)
    .bind(starts_at)
    .bind(ends_at)
    .bind(state.attestor_pubkey.clone())
    .bind(checkpoint_pda.to_string())
    .fetch_one(&state.db)
    .await
    .expect("failed to insert test checkpoint");

    // --- Fund a fresh participant keypair ---------------------------------
    let participant_seed: [u8; 32] = rand::random();
    let participant = SigningKey::from_bytes(&participant_seed);
    let participant_pubkey = Pubkey::new_from_array(participant.verifying_key().to_bytes());
    let participant_wallet = participant_pubkey.to_string();

    let fund_ix = system_instruction::transfer(&attestor_pubkey, &participant_pubkey, 10_000_000);
    let sig = state
        .blockchain
        .send_signed_by_attestor(&[fund_ix])
        .await
        .expect("funding transfer failed");
    assert!(state
        .blockchain
        .confirm(&sig, Duration::from_secs(30))
        .await
        .expect("RPC error confirming funding transfer"));

    // --- Mint a real rotating QR token via the production qr module ------
    let checkpoint_meta = qr::ActiveCheckpoint {
        event_id,
        rotation_seconds: 15,
    };
    let window = qr::get_or_create_current_qr(&state, checkpoint_id, &checkpoint_meta)
        .await
        .expect("failed to mint QR token");

    // --- validate -> prepare-transaction -> participant signs -> submitted
    let grant = handlers::validate(
        State(state.clone()),
        AuthenticatedWallet(participant_wallet.clone()),
        Json(ValidateRequest {
            qr_token: window.token.clone(),
            wallet: participant_wallet.clone(),
        }),
    )
    .await
    .expect("validate should succeed")
    .0;

    let prepared = handlers::prepare_transaction(
        State(state.clone()),
        AuthenticatedWallet(participant_wallet.clone()),
        Path(grant.grant_id),
    )
    .await
    .expect("prepare-transaction should succeed")
    .0;

    let signature = participant_sign_and_submit(&state, &participant, &prepared.transaction)
        .await
        .expect("check_in transaction should submit successfully");
    assert!(state
        .blockchain
        .confirm(&signature, Duration::from_secs(30))
        .await
        .expect("RPC error confirming check_in"));

    let submitted = handlers::submitted(
        State(state.clone()),
        AuthenticatedWallet(participant_wallet.clone()),
        Path(grant.grant_id),
        Json(SubmittedRequest {
            signature: signature.clone(),
        }),
    )
    .await
    .expect("submitted should succeed")
    .0;
    assert_eq!(submitted.status, "transaction_submitted");
    assert_eq!(
        submitted.transaction_signature.as_deref(),
        Some(signature.as_str())
    );

    println!("check_in signature: https://explorer.solana.com/tx/{signature}?cluster=devnet");

    // --- Ground truth: read the real on-chain state back ------------------
    let (participant_event_pda, _) = pda::participant_event(&event_pda, &participant_pubkey);
    let (attendance_pda, _) = pda::attendance(&event_pda, &checkpoint_pda, &participant_pubkey);

    let attendance = state
        .blockchain
        .fetch_account::<eventquest_chain::accounts::AttendanceAccount>(attendance_pda)
        .await
        .expect("RPC error fetching attendance")
        .expect("attendance account should exist after confirmation");
    assert_eq!(attendance.event, event_pda);
    assert_eq!(attendance.checkpoint, checkpoint_pda);
    assert_eq!(attendance.participant, participant_pubkey);
    assert_eq!(attendance.attestor, attestor_pubkey);
    assert_eq!(attendance.points_awarded, 50);

    let participant_event = state
        .blockchain
        .fetch_account::<eventquest_chain::accounts::ParticipantEventAccount>(participant_event_pda)
        .await
        .expect("RPC error fetching participant_event")
        .expect("participant_event account should exist (join_event should have run)");
    assert_eq!(participant_event.points, 50);
    assert_eq!(participant_event.checkin_count, 1);

    println!("attendance PDA: https://explorer.solana.com/address/{attendance_pda}?cluster=devnet");

    // --- Duplicate check-in must be rejected, both off-chain and on-chain
    let fresh_window = qr::get_or_create_current_qr(&state, checkpoint_id, &checkpoint_meta)
        .await
        .expect("failed to mint a second QR token");
    let duplicate_validate = handlers::validate(
        State(state.clone()),
        AuthenticatedWallet(participant_wallet.clone()),
        Json(ValidateRequest {
            qr_token: fresh_window.token,
            wallet: participant_wallet.clone(),
        }),
    )
    .await;
    assert!(
        duplicate_validate.is_err(),
        "a second check-in attempt for the same (checkpoint, participant) must be rejected off-chain"
    );

    // spec §0.9 steps 10-11: retry the exact same check_in instruction and
    // confirm Devnet itself rejects it (the `attendance` PDA's `init`
    // constraint — see programs/eventquest/src/instructions/check_in.rs).
    let challenge_hash: [u8; 32] = hex::decode(&grant.challenge_hash)
        .expect("hex")
        .try_into()
        .expect("32 bytes");
    let repeat_check_in_ix = Instruction {
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
    let repeat_tx = state
        .blockchain
        .build_and_co_sign(&[repeat_check_in_ix], participant_pubkey)
        .await
        .expect("building the repeat check_in transaction should still succeed");
    let repeat_result = participant_sign_and_submit(&state, &participant, &repeat_tx).await;
    assert!(
        repeat_result.is_err(),
        "resubmitting check_in for an existing Attendance PDA must fail on Devnet"
    );
    println!("duplicate check-in correctly rejected by Devnet: {repeat_result:?}");
}
