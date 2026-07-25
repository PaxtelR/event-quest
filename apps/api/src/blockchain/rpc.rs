//! Minimal Solana JSON-RPC client — only the methods this service needs
//! (spec §9.3/§9.5: build, submit, and confirm a transaction; §16: fetch
//! an account for reconciliation). See `blockchain/mod.rs` and this
//! crate's Cargo.toml for why `solana-rpc-client` isn't used here.
//!
//! Used by the check-ins module (task #18) via `BlockchainClient`.

use std::str::FromStr;
use std::time::Duration;

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use eventquest_domain::ErrorCode;
use serde::{de::DeserializeOwned, Deserialize};
use serde_json::{json, Value};
use solana_hash::Hash;

use crate::error::ApiError;

pub struct RpcClient {
    http: reqwest::Client,
    url: String,
}

#[derive(Deserialize)]
struct JsonRpcResponse<T> {
    result: Option<T>,
    error: Option<JsonRpcErrorBody>,
}

#[derive(Deserialize)]
struct JsonRpcErrorBody {
    code: i64,
    message: String,
}

enum RpcCallError {
    /// The endpoint returned JSON-RPC error code 429 — retry after a
    /// backoff instead of failing the whole request immediately.
    RateLimited,
    Other,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignatureStatus {
    // Part of the RPC response shape (needed for correct deserialization)
    // even though nothing reads them yet.
    #[allow(dead_code)]
    pub slot: u64,
    #[allow(dead_code)]
    pub confirmations: Option<u64>,
    pub err: Option<Value>,
    pub confirmation_status: Option<String>,
}

impl RpcClient {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            http: reqwest::Client::new(),
            url: url.into(),
        }
    }

    async fn call<T: DeserializeOwned>(&self, method: &str, params: Value) -> Result<T, ApiError> {
        // Public Devnet RPC endpoints rate-limit per method (observed: HTTP
        // 429 surfaced as a JSON-RPC error, not a transport failure) well
        // within normal traffic during a burst of calls — e.g. this
        // module's own check-in flow issuing several RPC calls a second
        // apart. A short, bounded retry absorbs that without the caller
        // having to know about it.
        const MAX_ATTEMPTS: u32 = 6;
        let mut delay = Duration::from_millis(500);

        for attempt in 1..=MAX_ATTEMPTS {
            match self.try_call(method, &params).await {
                Ok(value) => return Ok(value),
                Err(RpcCallError::RateLimited) if attempt < MAX_ATTEMPTS => {
                    tokio::time::sleep(delay).await;
                    delay *= 2;
                }
                Err(_) => return Err(ApiError::new(ErrorCode::ChainUnavailable)),
            }
        }
        Err(ApiError::new(ErrorCode::ChainUnavailable))
    }

    async fn try_call<T: DeserializeOwned>(
        &self,
        method: &str,
        params: &Value,
    ) -> Result<T, RpcCallError> {
        let body = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params,
        });

        let response = self
            .http
            .post(&self.url)
            .json(&body)
            .send()
            .await
            .map_err(|error| {
                tracing::error!(%error, method, "RPC request failed");
                RpcCallError::Other
            })?;

        let parsed: JsonRpcResponse<T> = response.json().await.map_err(|error| {
            tracing::error!(%error, method, "RPC response could not be parsed");
            RpcCallError::Other
        })?;

        if let Some(err) = parsed.error {
            if err.code == 429 {
                tracing::warn!(method, "RPC rate-limited, retrying");
                return Err(RpcCallError::RateLimited);
            }
            tracing::error!(code = err.code, message = %err.message, method, "RPC returned an error");
            return Err(RpcCallError::Other);
        }

        parsed.result.ok_or(RpcCallError::Other)
    }

    pub async fn get_latest_blockhash(&self) -> Result<Hash, ApiError> {
        #[derive(Deserialize)]
        struct BlockhashValue {
            blockhash: String,
        }
        #[derive(Deserialize)]
        struct Resp {
            value: BlockhashValue,
        }

        let resp: Resp = self
            .call("getLatestBlockhash", json!([{ "commitment": "confirmed" }]))
            .await?;

        Hash::from_str(&resp.value.blockhash).map_err(|error| {
            tracing::error!(%error, "failed to parse blockhash from RPC response");
            ApiError::new(ErrorCode::ChainUnavailable)
        })
    }

    /// Submits a base64-encoded, fully-signed transaction. Returns the
    /// transaction signature. Does not wait for confirmation — see
    /// `confirm_transaction`.
    pub async fn send_transaction(&self, tx_base64: &str) -> Result<String, ApiError> {
        self.call(
            "sendTransaction",
            json!([
                tx_base64,
                {
                    "encoding": "base64",
                    "skipPreflight": false,
                    "preflightCommitment": "confirmed",
                    "maxRetries": 5,
                }
            ]),
        )
        .await
    }

    pub async fn get_signature_statuses(
        &self,
        signatures: &[String],
    ) -> Result<Vec<Option<SignatureStatus>>, ApiError> {
        #[derive(Deserialize)]
        struct Resp {
            value: Vec<Option<SignatureStatus>>,
        }

        let resp: Resp = self
            .call(
                "getSignatureStatuses",
                json!([signatures, { "searchTransactionHistory": true }]),
            )
            .await?;

        Ok(resp.value)
    }

    /// Fetches raw account data (base64-decoded), or `None` if the
    /// account doesn't exist. Owner/lamports aren't needed by any current
    /// caller, so only data is returned.
    pub async fn get_account_data(&self, pubkey_base58: &str) -> Result<Option<Vec<u8>>, ApiError> {
        #[derive(Deserialize)]
        struct AccountValue {
            data: (String, String),
        }
        #[derive(Deserialize)]
        struct Resp {
            value: Option<AccountValue>,
        }

        let resp: Resp = self
            .call(
                "getAccountInfo",
                json!([pubkey_base58, { "encoding": "base64", "commitment": "confirmed" }]),
            )
            .await?;

        match resp.value {
            Some(account) => {
                let bytes = BASE64.decode(account.data.0).map_err(|error| {
                    tracing::error!(%error, "failed to base64-decode account data");
                    ApiError::new(ErrorCode::ChainUnavailable)
                })?;
                Ok(Some(bytes))
            }
            None => Ok(None),
        }
    }
}
