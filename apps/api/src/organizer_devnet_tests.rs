//! Real end-to-end proof that the organizer on-chain-provisioning gap
//! (see `checkins::mod`'s header comment) is actually closed: a fresh
//! organizer keypair creates an event and a checkpoint through the real
//! `events`/`checkpoints` handlers, signs and submits the real prepared
//! `initialize_event`+`update_event_status`/`create_checkpoint`
//! transactions itself (not the attestor, and not a test-only shortcut —
//! exactly the wallet flow a real frontend would drive), and only then is
//! a participant's check-in run through the real `checkins` handlers
//! against that now-genuinely-provisioned event/checkpoint. Also exercises
//! `pause` against an already-provisioned event, the other branch of
//! `events::handlers::prepare_status_change`.
//!
//! Ignored by default — `cargo test -p eventquest-api -- --ignored
//! organizer_devnet_tests`. Requires: `docker-compose up` (Postgres +
//! Redis), a Devnet RPC, and `ATTESTOR_KEYPAIR_PATH` funded with Devnet
//! SOL (to fund the fresh organizer/participant keypairs this test
//! generates).

use std::time::Duration;

use anchor_lang::prelude::Pubkey;
use anchor_lang::solana_program::system_instruction;
use axum::extract::{Path, State};
use axum::Json;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use chrono::Utc;
use ed25519_dalek::{Signer as DalekSigner, SigningKey};
use eventquest_config::{AppConfig, QrConfig, SessionConfig, SolanaConfig};
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

use crate::auth::AuthenticatedWallet;
use crate::blockchain::BlockchainClient;
use crate::checkins;
use crate::checkpoints;
use crate::events;
use crate::onchain::SubmittedRequest;
use crate::qr;
use crate::state::AppState;

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
    let program_id = eventquest_chain::ID;

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

    let blockchain = BlockchainClient::new(&rpc_http_url, &attestor_keypair_path, program_id)
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
            program_id,
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

/// Signs `tx_base64` as its sole signer (fee payer, always signature index
/// 0) and submits it — used for both the organizer's and the
/// participant's own transactions in this test, exactly like a wallet
/// would.
async fn sign_and_submit(state: &AppState, signer: &SigningKey, tx_base64: &str) -> String {
    let tx_bytes = BASE64.decode(tx_base64).expect("valid base64");
    let mut transaction: solana_transaction::Transaction =
        bincode::deserialize(&tx_bytes).expect("valid transaction bytes");
    let message_bytes = transaction.message_data();
    let signature_bytes = signer.sign(&message_bytes).to_bytes();
    transaction.signatures[0] = solana_signature::Signature::from(signature_bytes);
    let signed_bytes = bincode::serialize(&transaction).expect("transaction always serializes");
    let signature = state
        .blockchain
        .submit(&BASE64.encode(signed_bytes))
        .await
        .expect("transaction should submit successfully");
    assert!(
        state
            .blockchain
            .confirm(&signature, Duration::from_secs(30))
            .await
            .expect("RPC error while confirming"),
        "transaction {signature} did not confirm within 30s"
    );
    signature
}

async fn fund(state: &AppState, to: Pubkey, lamports: u64) {
    let instruction =
        system_instruction::transfer(&state.blockchain.attestor_pubkey, &to, lamports);
    let signature = state
        .blockchain
        .send_signed_by_attestor(&[instruction])
        .await
        .expect("funding transfer failed");
    assert!(state
        .blockchain
        .confirm(&signature, Duration::from_secs(30))
        .await
        .expect("RPC error while confirming funding transfer"));
}

#[tokio::test]
#[ignore = "hits live Solana Devnet + local Postgres/Redis; requires docker-compose up and a funded ATTESTOR_KEYPAIR_PATH"]
async fn organizer_provisions_event_and_checkpoint_then_a_participant_checks_in() {
    let state = build_test_state().await;

    let organizer_seed: [u8; 32] = rand::random();
    let organizer = SigningKey::from_bytes(&organizer_seed);
    let organizer_pubkey = Pubkey::new_from_array(organizer.verifying_key().to_bytes());
    let organizer_wallet = organizer_pubkey.to_string();
    fund(&state, organizer_pubkey, 20_000_000).await;

    let organization_id: Uuid =
        sqlx::query_scalar("insert into organizations (name, slug) values ($1, $2) returning id")
            .bind("Organizer Devnet Test Org")
            .bind(format!("organizer-devnet-test-org-{}", Uuid::new_v4()))
            .fetch_one(&state.db)
            .await
            .expect("failed to insert test organization");
    sqlx::query(
        "insert into organization_members (organization_id, wallet_address, role) values ($1, $2, 'owner')",
    )
    .bind(organization_id)
    .bind(&organizer_wallet)
    .execute(&state.db)
    .await
    .expect("failed to insert organization_members row");

    // --- Create + publish the event through the real API handlers ------
    let now = Utc::now();
    let event = events::handlers::create_event(
        State(state.clone()),
        AuthenticatedWallet(organizer_wallet.clone()),
        Json(events::models::CreateEventRequest {
            name: "Organizer Devnet Test Event".to_string(),
            slug: format!("organizer-devnet-test-event-{}", Uuid::new_v4()),
            description: None,
            banner_url: None,
            location_name: None,
            location_address: None,
            timezone: None,
            starts_at: now - chrono::Duration::hours(1),
            ends_at: now + chrono::Duration::hours(1),
            visibility: None,
            organization_id: Some(organization_id),
        }),
    )
    .await
    .expect("create_event should succeed")
    .1
     .0;
    assert_eq!(event.status, "draft");
    assert!(event.onchain_event_address.is_none());

    let prepare_publish = events::handlers::prepare_publish(
        State(state.clone()),
        AuthenticatedWallet(organizer_wallet.clone()),
        Path(event.id),
    )
    .await
    .expect("prepare_publish should succeed")
    .0;
    assert!(
        prepare_publish.requires_signature,
        "a never-provisioned event must require the organizer's signature to publish"
    );
    let publish_tx = prepare_publish
        .transaction
        .expect("requires_signature implies a transaction is present");

    let publish_signature = sign_and_submit(&state, &organizer, &publish_tx).await;
    println!(
        "publish (initialize_event + update_event_status) signature: \
         https://explorer.solana.com/tx/{publish_signature}?cluster=devnet"
    );

    let published_event = events::handlers::publish_submitted(
        State(state.clone()),
        AuthenticatedWallet(organizer_wallet.clone()),
        Path(event.id),
        Json(SubmittedRequest {
            signature: publish_signature,
        }),
    )
    .await
    .expect("publish_submitted should succeed")
    .0;
    assert_eq!(published_event.status, "active");
    let event_pda: Pubkey = published_event
        .onchain_event_address
        .as_deref()
        .expect("onchain_event_address should be set after publish")
        .parse()
        .expect("valid pubkey");

    let event_account = state
        .blockchain
        .fetch_account::<eventquest_chain::accounts::EventAccount>(event_pda)
        .await
        .expect("RPC error fetching event account")
        .expect("event account should exist on-chain after publish");
    assert_eq!(event_account.authority, organizer_pubkey);
    println!("event PDA: https://explorer.solana.com/address/{event_pda}?cluster=devnet");

    // --- Create + activate the checkpoint through the real API handlers -
    let checkpoint = checkpoints::handlers::create_checkpoint(
        State(state.clone()),
        AuthenticatedWallet(organizer_wallet.clone()),
        Path(event.id),
        Json(checkpoints::models::CreateCheckpointRequest {
            name: "Organizer Devnet Test Checkpoint".to_string(),
            description: None,
            points: 40,
            rotation_seconds: Some(15),
            opens_at: now - chrono::Duration::hours(1),
            closes_at: now + chrono::Duration::hours(1),
        }),
    )
    .await
    .expect("create_checkpoint should succeed")
    .1
     .0;
    assert!(checkpoint.onchain_checkpoint_address.is_none());

    let prepare_activate = checkpoints::handlers::prepare_activate(
        State(state.clone()),
        AuthenticatedWallet(organizer_wallet.clone()),
        Path(checkpoint.id),
    )
    .await
    .expect("prepare_activate should succeed")
    .0;
    assert!(prepare_activate.requires_signature);
    let activate_tx = prepare_activate.transaction.expect("transaction present");
    let activate_signature = sign_and_submit(&state, &organizer, &activate_tx).await;
    println!(
        "create_checkpoint signature: https://explorer.solana.com/tx/{activate_signature}?cluster=devnet"
    );

    let activated_checkpoint = checkpoints::handlers::activate_submitted(
        State(state.clone()),
        AuthenticatedWallet(organizer_wallet.clone()),
        Path(checkpoint.id),
        Json(SubmittedRequest {
            signature: activate_signature,
        }),
    )
    .await
    .expect("activate_submitted should succeed")
    .0;
    assert_eq!(activated_checkpoint.status, "active");
    let checkpoint_pda: Pubkey = activated_checkpoint
        .onchain_checkpoint_address
        .as_deref()
        .expect("onchain_checkpoint_address should be set after activation")
        .parse()
        .expect("valid pubkey");
    println!("checkpoint PDA: https://explorer.solana.com/address/{checkpoint_pda}?cluster=devnet");

    // --- Exercise `pause`, the "already provisioned" branch -------------
    let prepare_pause = events::handlers::prepare_pause(
        State(state.clone()),
        AuthenticatedWallet(organizer_wallet.clone()),
        Path(event.id),
    )
    .await
    .expect("prepare_pause should succeed")
    .0;
    assert!(
        prepare_pause.requires_signature,
        "pausing an already-provisioned event must still require a signature"
    );
    let pause_tx = prepare_pause.transaction.expect("transaction present");
    let pause_signature = sign_and_submit(&state, &organizer, &pause_tx).await;
    let paused_event = events::handlers::pause_submitted(
        State(state.clone()),
        AuthenticatedWallet(organizer_wallet.clone()),
        Path(event.id),
        Json(SubmittedRequest {
            signature: pause_signature,
        }),
    )
    .await
    .expect("pause_submitted should succeed")
    .0;
    assert_eq!(paused_event.status, "paused");

    // Re-publish (paused -> active) so the participant check-in below has
    // an active event to check into — also proves the "already
    // provisioned" branch of `prepare_publish` (no `initialize_event`
    // bundled this time).
    let republish = events::handlers::prepare_publish(
        State(state.clone()),
        AuthenticatedWallet(organizer_wallet.clone()),
        Path(event.id),
    )
    .await
    .expect("re-publish prepare should succeed")
    .0;
    assert!(republish.requires_signature);
    let republish_tx = republish.transaction.expect("transaction present");
    let republish_signature = sign_and_submit(&state, &organizer, &republish_tx).await;
    let reactivated_event = events::handlers::publish_submitted(
        State(state.clone()),
        AuthenticatedWallet(organizer_wallet.clone()),
        Path(event.id),
        Json(SubmittedRequest {
            signature: republish_signature,
        }),
    )
    .await
    .expect("re-publish submitted should succeed")
    .0;
    assert_eq!(reactivated_event.status, "active");

    // --- Now prove the gap is actually closed: a participant checks in
    // against an event/checkpoint that went through the *real* organizer
    // API flow above, not a test-only on-chain shortcut. ------------------
    let participant_seed: [u8; 32] = rand::random();
    let participant = SigningKey::from_bytes(&participant_seed);
    let participant_pubkey = Pubkey::new_from_array(participant.verifying_key().to_bytes());
    let participant_wallet = participant_pubkey.to_string();
    fund(&state, participant_pubkey, 10_000_000).await;

    let checkpoint_meta = qr::ActiveCheckpoint {
        event_id: event.id,
        rotation_seconds: 15,
    };
    let window = qr::get_or_create_current_qr(&state, checkpoint.id, &checkpoint_meta)
        .await
        .expect("failed to mint QR token");

    let grant = checkins::handlers::validate(
        State(state.clone()),
        AuthenticatedWallet(participant_wallet.clone()),
        Json(checkins::models::ValidateRequest {
            qr_token: window.token,
            wallet: participant_wallet.clone(),
        }),
    )
    .await
    .expect("validate should succeed")
    .0;

    let prepared = checkins::handlers::prepare_transaction(
        State(state.clone()),
        AuthenticatedWallet(participant_wallet.clone()),
        Path(grant.grant_id),
    )
    .await
    .expect("prepare-transaction should succeed")
    .0;

    let check_in_signature = sign_and_submit(&state, &participant, &prepared.transaction).await;

    let submitted = checkins::handlers::submitted(
        State(state.clone()),
        AuthenticatedWallet(participant_wallet.clone()),
        Path(grant.grant_id),
        Json(checkins::models::SubmittedRequest {
            signature: check_in_signature.clone(),
        }),
    )
    .await
    .expect("submitted should succeed")
    .0;
    assert_eq!(submitted.status, "transaction_submitted");

    let (participant_event_pda, _) =
        eventquest_chain::pda::participant_event(&event_pda, &participant_pubkey);
    let (attendance_pda, _) =
        eventquest_chain::pda::attendance(&event_pda, &checkpoint_pda, &participant_pubkey);

    let attendance = state
        .blockchain
        .fetch_account::<eventquest_chain::accounts::AttendanceAccount>(attendance_pda)
        .await
        .expect("RPC error fetching attendance")
        .expect("attendance account should exist after confirmation");
    assert_eq!(attendance.event, event_pda);
    assert_eq!(attendance.checkpoint, checkpoint_pda);
    assert_eq!(attendance.participant, participant_pubkey);
    assert_eq!(attendance.points_awarded, 40);

    let participant_event = state
        .blockchain
        .fetch_account::<eventquest_chain::accounts::ParticipantEventAccount>(participant_event_pda)
        .await
        .expect("RPC error fetching participant_event")
        .expect("participant_event account should exist (join_event should have run)");
    assert_eq!(participant_event.points, 40);

    println!(
        "check_in signature: https://explorer.solana.com/tx/{check_in_signature}?cluster=devnet"
    );
    println!("attendance PDA: https://explorer.solana.com/address/{attendance_pda}?cluster=devnet");
}
