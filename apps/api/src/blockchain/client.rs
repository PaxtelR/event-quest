//! Ties together instruction building (via `crates/eventquest-chain`'s
//! generated client), attestor co-signing, transaction submission, and
//! confirmation polling — spec §9.3 ("Por que usar o attestor como
//! coassinante") and §9.5. Used by the check-ins module (task #18).

use std::str::FromStr;
use std::time::Duration;

use anchor_lang::prelude::Pubkey;
use anchor_lang::solana_program::instruction::Instruction;
use anchor_lang::AccountDeserialize;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use ed25519_dalek::{Signer as DalekSigner, SigningKey};
use eventquest_domain::ErrorCode;
use serde_json::Value;
use solana_message::Message;
use solana_signature::Signature;
use solana_transaction::Transaction;

use crate::error::ApiError;

use super::rpc::{RpcClient, SignatureStatus};

pub struct BlockchainClient {
    rpc: RpcClient,
    attestor: SigningKey,
    pub attestor_pubkey: Pubkey,
    pub program_id: Pubkey,
}

impl BlockchainClient {
    pub async fn new(
        rpc_url: &str,
        attestor_keypair_path: &str,
        program_id: Pubkey,
    ) -> anyhow::Result<Self> {
        let file_bytes = tokio::fs::read(attestor_keypair_path).await?;
        let keypair_bytes: Vec<u8> = serde_json::from_slice(&file_bytes)?;
        if keypair_bytes.len() != 64 {
            anyhow::bail!(
                "attestor keypair file at {attestor_keypair_path} is not a 64-byte solana-keygen keypair"
            );
        }

        let seed: [u8; 32] = keypair_bytes[0..32].try_into().expect("checked len above");
        let attestor = SigningKey::from_bytes(&seed);

        let pubkey_bytes: [u8; 32] = keypair_bytes[32..64].try_into().expect("checked len above");
        let attestor_pubkey = Pubkey::new_from_array(pubkey_bytes);

        Ok(Self {
            rpc: RpcClient::new(rpc_url),
            attestor,
            attestor_pubkey,
            program_id,
        })
    }

    /// Builds a transaction from `instructions`, fetches a fresh
    /// blockhash, co-signs as the attestor, and returns it base64-encoded
    /// — ready for the participant to sign with their wallet and submit
    /// (spec §9.3's `prepare-transaction` response shape). Fails if the
    /// attestor isn't actually a required signer of these instructions —
    /// that would silently produce an unusable transaction otherwise.
    pub async fn build_and_co_sign(
        &self,
        instructions: &[Instruction],
        fee_payer: Pubkey,
    ) -> Result<String, ApiError> {
        let blockhash = self.rpc.get_latest_blockhash().await?;
        let message = Message::new_with_blockhash(instructions, Some(&fee_payer), &blockhash);

        let attestor_index = message
            .signer_keys()
            .iter()
            .position(|&&key| key == self.attestor_pubkey)
            .ok_or_else(|| {
                tracing::error!(
                    attestor = %self.attestor_pubkey,
                    "attestor is not a required signer of the instruction set"
                );
                ApiError::new(ErrorCode::InternalError)
            })?;

        let mut transaction = Transaction::new_unsigned(message);
        let message_bytes = transaction.message_data();
        let signature_bytes = self.attestor.sign(&message_bytes).to_bytes();
        transaction.signatures[attestor_index] = Signature::from(signature_bytes);

        let serialized = bincode::serialize(&transaction)
            .map_err(|_| ApiError::new(ErrorCode::InternalError))?;
        Ok(BASE64.encode(serialized))
    }

    /// Builds an unsigned transaction with `fee_payer` as the sole
    /// intended signer and no backend co-signature — used for
    /// organizer-signed on-chain actions (`onchain::build_organizer_tx`)
    /// where, unlike check-ins, the backend holds no relevant key at all.
    pub async fn build_unsigned(
        &self,
        instructions: &[Instruction],
        fee_payer: Pubkey,
    ) -> Result<String, ApiError> {
        let blockhash = self.rpc.get_latest_blockhash().await?;
        let message = Message::new_with_blockhash(instructions, Some(&fee_payer), &blockhash);
        let transaction = Transaction::new_unsigned(message);
        let serialized = bincode::serialize(&transaction)
            .map_err(|_| ApiError::new(ErrorCode::InternalError))?;
        Ok(BASE64.encode(serialized))
    }

    /// Submits an already fully-signed, base64-encoded transaction.
    ///
    /// In production this is never called on the participant's own
    /// check-in transaction: spec §9.4 has the *frontend* submit and
    /// monitor it directly against the participant's own wallet/RPC
    /// connection, since the backend's `submitted` endpoint only receives
    /// the resulting signature after the fact. It's a real, independently
    /// useful method regardless — `send_signed_by_attestor`'s tests use it
    /// for backend-controlled (attestor-signed) transactions, and it's the
    /// natural building block for the Devnet smoke-test script (spec §0.9,
    /// Phase 6) or any future admin-triggered on-chain action.
    #[allow(dead_code)]
    pub async fn submit(&self, signed_tx_base64: &str) -> Result<String, ApiError> {
        self.rpc.send_transaction(signed_tx_base64).await
    }

    /// Polls `getSignatureStatuses` until the transaction reaches at least
    /// `confirmed` commitment, fails, or `timeout` elapses. Check-ins'
    /// `submitted` handler deliberately doesn't use this (the indexer,
    /// spec §16, is the actual source of truth for attendance, so a
    /// lighter-weight one-shot `signature_error` check is enough there)
    /// — but the organizer on-chain-provisioning flow (`onchain` module)
    /// does: those off-chain status transitions are gated on real
    /// confirmation, since there's no separate indexer watching for them.
    pub async fn confirm(&self, signature: &str, timeout: Duration) -> Result<bool, ApiError> {
        let deadline = tokio::time::Instant::now() + timeout;
        let signatures = vec![signature.to_string()];

        loop {
            let statuses = self.rpc.get_signature_statuses(&signatures).await?;
            if let Some(Some(status)) = statuses.into_iter().next() {
                if let Some(err) = &status.err {
                    tracing::warn!(signature, ?err, "transaction failed on-chain");
                    return Ok(false);
                }
                if is_confirmed_or_better(&status) {
                    return Ok(true);
                }
            }

            if tokio::time::Instant::now() >= deadline {
                return Ok(false);
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }

    /// Builds, fully signs (as the attestor — the sole required signer),
    /// submits, and returns the signature for `instructions`. Not part of
    /// the participant-facing check-in flow (which needs the participant's
    /// own signature too — see `build_and_co_sign`); used by
    /// `checkins::devnet_tests` to provision an event/checkpoint on-chain
    /// with the attestor standing in for the organizer's wallet, exactly
    /// like `devnet_tests::initialize_event_on_devnet_and_read_it_back`
    /// below already does for a single `initialize_event` call.
    #[cfg(test)]
    pub(crate) async fn send_signed_by_attestor(
        &self,
        instructions: &[Instruction],
    ) -> Result<String, ApiError> {
        let blockhash = self.rpc.get_latest_blockhash().await?;
        let message =
            Message::new_with_blockhash(instructions, Some(&self.attestor_pubkey), &blockhash);
        let mut transaction = Transaction::new_unsigned(message);
        let message_bytes = transaction.message_data();
        let signature_bytes = self.attestor.sign(&message_bytes).to_bytes();
        // The fee payer is always placed at signer index 0 by
        // `Message::new_with_blockhash`, and the attestor is the fee payer
        // here (and the only required signer for the instructions this is
        // used with).
        transaction.signatures[0] = Signature::from(signature_bytes);
        let serialized = bincode::serialize(&transaction)
            .map_err(|_| ApiError::new(ErrorCode::InternalError))?;
        self.submit(&BASE64.encode(serialized)).await
    }

    /// One-shot check for whether `signature` has already failed on-chain
    /// (`err` present in `getSignatureStatuses`) — does not wait or poll.
    /// Used by the check-ins `submitted` endpoint (spec §9.5) to give an
    /// immediate failure signal without blocking the request on full
    /// confirmation, which is the indexer's job (spec §16): a `None` here
    /// means "no failure observed yet", not "confirmed".
    pub async fn signature_error(&self, signature: &str) -> Result<Option<Value>, ApiError> {
        let signatures = vec![signature.to_string()];
        let statuses = self.rpc.get_signature_statuses(&signatures).await?;
        Ok(statuses.into_iter().next().flatten().and_then(|s| s.err))
    }

    /// Fetches and Borsh/Anchor-decodes an account — used for reading
    /// back Event/Checkpoint/Attendance PDAs.
    pub async fn fetch_account<T: AccountDeserialize>(
        &self,
        pubkey: Pubkey,
    ) -> Result<Option<T>, ApiError> {
        let data = self.rpc.get_account_data(&pubkey.to_string()).await?;
        match data {
            Some(bytes) => {
                let account = T::try_deserialize(&mut bytes.as_slice()).map_err(|error| {
                    tracing::error!(%error, %pubkey, "failed to decode account data");
                    ApiError::new(ErrorCode::InternalError)
                })?;
                Ok(Some(account))
            }
            None => Ok(None),
        }
    }
}

#[allow(dead_code)]
fn is_confirmed_or_better(status: &SignatureStatus) -> bool {
    matches!(
        status.confirmation_status.as_deref(),
        Some("confirmed") | Some("finalized")
    )
}

/// Parses a base58 Solana address into `anchor_lang`'s `Pubkey` type —
/// used at API boundaries (request bodies, config) where addresses arrive
/// as plain strings. Not called yet — the check-ins module (task #18) is
/// the first handler that needs to parse a wallet address from a request.
pub fn parse_pubkey(base58: &str) -> Result<Pubkey, ApiError> {
    Pubkey::from_str(base58)
        .map_err(|_| ApiError::with_message(ErrorCode::ValidationError, "invalid Solana address"))
}

#[cfg(test)]
mod devnet_tests {
    //! Real end-to-end proof against the deployed Devnet program —
    //! `cargo test -p eventquest-api -- --ignored blockchain::client::devnet_tests`.
    //! Ignored by default (network + a funded Devnet keypair required) so
    //! `cargo test` stays hermetic; this is the evidence, not a mock.

    use anchor_lang::InstructionData;
    use eventquest_chain::{client::accounts, client::args, pda};

    use super::*;

    fn env_or_skip(name: &str) -> Option<String> {
        std::env::var(name).ok()
    }

    #[tokio::test]
    #[ignore = "hits live Solana Devnet; requires ATTESTOR_KEYPAIR_PATH funded with Devnet SOL"]
    async fn initialize_event_on_devnet_and_read_it_back() {
        let rpc_url = env_or_skip("SOLANA_RPC_HTTP_URL")
            .unwrap_or_else(|| "https://api.devnet.solana.com".to_string());
        let attestor_keypair_path = env_or_skip("ATTESTOR_KEYPAIR_PATH")
            .unwrap_or_else(|| ".secrets/attestor-devnet.json".to_string());
        let program_id = Pubkey::from_str("CuULC7T6d49mf89sRPU8WLeYfkF74kDyEKvfCe2rXq8H").unwrap();

        let client = BlockchainClient::new(&rpc_url, &attestor_keypair_path, program_id)
            .await
            .expect("failed to build BlockchainClient — is the attestor keypair funded?");

        // Reuse the attestor as the event's authority too, so this test
        // needs only one funded keypair — real usage always has the
        // organizer's own wallet as authority.
        let authority = client.attestor_pubkey;
        let external_id_hash: [u8; 32] = rand::random();
        let (event_pda, _bump) = pda::event(&authority, &external_id_hash);

        let now = chrono::Utc::now().timestamp();
        let ix_accounts = accounts::InitializeEvent {
            authority,
            event: event_pda,
            system_program: anchor_lang::system_program::ID,
        };
        let ix_data = args::InitializeEvent {
            external_id_hash,
            starts_at: now,
            ends_at: now + 3600,
        };
        let instruction = Instruction {
            program_id,
            accounts: anchor_lang::ToAccountMetas::to_account_metas(&ix_accounts, None),
            data: ix_data.data(),
        };

        let signed_tx_base64 = client
            .build_and_co_sign(&[instruction], authority)
            .await
            .expect("failed to build/sign initialize_event transaction");

        let signature = client
            .submit(&signed_tx_base64)
            .await
            .expect("failed to submit initialize_event transaction");
        println!(
            "initialize_event signature: https://explorer.solana.com/tx/{signature}?cluster=devnet"
        );

        let confirmed = client
            .confirm(&signature, Duration::from_secs(30))
            .await
            .expect("RPC error while confirming");
        assert!(confirmed, "transaction did not confirm within 30s");

        let event_account = client
            .fetch_account::<eventquest_chain::accounts::EventAccount>(event_pda)
            .await
            .expect("RPC error while fetching event account")
            .expect("event account should exist after confirmation");

        assert_eq!(event_account.authority, authority);
        assert_eq!(event_account.external_id_hash, external_id_hash);
        assert_eq!(event_account.checkpoint_count, 0);
        println!("event PDA: https://explorer.solana.com/address/{event_pda}?cluster=devnet");
    }
}
