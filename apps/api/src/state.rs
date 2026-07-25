use std::ops::Deref;
use std::sync::Arc;

use eventquest_config::AppConfig;
use sqlx::PgPool;

use crate::blockchain::BlockchainClient;

/// Shared application state, cheaply `Clone`-able (an `Arc` underneath) so
/// axum can hand a copy to every handler.
#[derive(Clone)]
pub struct AppState(Arc<AppStateInner>);

pub struct AppStateInner {
    pub config: AppConfig,
    pub db: PgPool,
    pub redis: redis::aio::ConnectionManager,
    /// Base58 public key of the checkpoint attestor — kept alongside
    /// `blockchain` (which also loads it) so handlers that only need the
    /// pubkey (checkpoint creation) don't have to touch signing machinery.
    pub attestor_pubkey: String,
    pub blockchain: BlockchainClient,
}

impl AppState {
    pub fn new(
        config: AppConfig,
        db: PgPool,
        redis: redis::aio::ConnectionManager,
        attestor_pubkey: String,
        blockchain: BlockchainClient,
    ) -> Self {
        Self(Arc::new(AppStateInner {
            config,
            db,
            redis,
            attestor_pubkey,
            blockchain,
        }))
    }
}

impl Deref for AppState {
    type Target = AppStateInner;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Reads a solana-keygen JSON keypair file (`[u8; 64]` = 32-byte seed +
/// 32-byte public key) and returns just the base58-encoded public key.
pub async fn load_attestor_pubkey(path: &str) -> anyhow::Result<String> {
    let bytes = tokio::fs::read(path).await?;
    let keypair: Vec<u8> = serde_json::from_slice(&bytes)?;
    let public_key_bytes: [u8; 32] = keypair
        .get(32..64)
        .ok_or_else(|| anyhow::anyhow!("attestor keypair file at {path} is not 64 bytes"))?
        .try_into()
        .expect("slice of length 32");
    Ok(bs58::encode(public_key_bytes).into_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn loads_pubkey_from_a_real_keygen_file() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!(
            "eventquest-test-keypair-{}.json",
            uuid::Uuid::new_v4()
        ));

        // A syntactically valid 64-byte keypair (doesn't need to be a real
        // signing key for this test — only the byte-slicing is exercised).
        let bytes: Vec<u8> = (0u8..64).collect();
        tokio::fs::write(&path, serde_json::to_vec(&bytes).unwrap())
            .await
            .unwrap();

        let pubkey = load_attestor_pubkey(path.to_str().unwrap()).await.unwrap();
        let expected: [u8; 32] = bytes[32..64].try_into().unwrap();
        assert_eq!(pubkey, bs58::encode(expected).into_string());

        tokio::fs::remove_file(&path).await.ok();
    }
}
