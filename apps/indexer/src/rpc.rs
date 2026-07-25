//! Minimal Solana JSON-RPC client — only the two read-only methods this
//! worker needs (spec §16: walk the program's signatures from a cursor,
//! then fetch each transaction's logs to decode `AttendanceRecorded`).
//! Mirrors `apps/api/src/blockchain/rpc.rs`'s approach (including the
//! bounded retry for rate-limited public Devnet endpoints) rather than
//! sharing code with it — the two services need different, small, mostly
//! non-overlapping RPC surfaces, and duplicating ~80 lines here is cheaper
//! than introducing a shared crate for it.

use std::time::Duration;

use anyhow::{anyhow, Result};
use serde::{de::DeserializeOwned, Deserialize};
use serde_json::{json, Value};

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

#[derive(Debug, Clone, Deserialize)]
pub struct SignatureInfo {
    pub signature: String,
    pub slot: u64,
    pub err: Option<Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TransactionMeta {
    pub err: Option<Value>,
    #[serde(rename = "logMessages")]
    pub log_messages: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TransactionInfo {
    pub slot: u64,
    #[serde(rename = "blockTime")]
    pub block_time: Option<i64>,
    pub meta: Option<TransactionMeta>,
}

enum RpcCallError {
    RateLimited,
    Other(String),
}

impl RpcClient {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            http: reqwest::Client::new(),
            url: url.into(),
        }
    }

    async fn call<T: DeserializeOwned>(&self, method: &str, params: Value) -> Result<T> {
        const MAX_ATTEMPTS: u32 = 6;
        let mut delay = Duration::from_millis(500);

        for attempt in 1..=MAX_ATTEMPTS {
            match self.try_call(method, &params).await {
                Ok(value) => return Ok(value),
                Err(RpcCallError::RateLimited) if attempt < MAX_ATTEMPTS => {
                    tracing::warn!(method, attempt, "RPC rate-limited, retrying");
                    tokio::time::sleep(delay).await;
                    delay *= 2;
                }
                Err(RpcCallError::RateLimited) => {
                    return Err(anyhow!(
                        "RPC call to {method} failed: rate-limited after retries"
                    ));
                }
                Err(RpcCallError::Other(message)) => {
                    return Err(anyhow!("RPC call to {method} failed: {message}"));
                }
            }
        }
        Err(anyhow!("RPC call to {method} failed after retries"))
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
            .map_err(|error| RpcCallError::Other(error.to_string()))?;

        let parsed: JsonRpcResponse<T> = response
            .json()
            .await
            .map_err(|error| RpcCallError::Other(error.to_string()))?;

        if let Some(err) = parsed.error {
            if err.code == 429 {
                return Err(RpcCallError::RateLimited);
            }
            return Err(RpcCallError::Other(format!(
                "{} (code {})",
                err.message, err.code
            )));
        }

        parsed
            .result
            .ok_or_else(|| RpcCallError::Other("empty RPC result".to_string()))
    }

    /// Signatures involving `address`, newest-first — spec §16 step 2.
    /// `before` pages backward past the newest page when more than one
    /// page of new signatures exists since the last tick. `commitment`
    /// distinguishes the frequent fast-path tick (`confirmed`) from the
    /// periodic wider reconciliation pass (`finalized` — spec §16's
    /// suggested commitment for closing/reports and the practical mitigation
    /// for reorgs, since finalized state never reverts).
    pub async fn get_signatures_for_address(
        &self,
        address: &str,
        limit: usize,
        before: Option<&str>,
        commitment: &str,
    ) -> Result<Vec<SignatureInfo>> {
        let mut opts = serde_json::Map::new();
        opts.insert("limit".to_string(), json!(limit));
        opts.insert("commitment".to_string(), json!(commitment));
        if let Some(before) = before {
            opts.insert("before".to_string(), json!(before));
        }
        self.call("getSignaturesForAddress", json!([address, opts]))
            .await
    }

    /// Fetches a transaction's logs at the given commitment — spec §16
    /// step 3 ("Decodifica eventos AttendanceRecorded").
    pub async fn get_transaction(
        &self,
        signature: &str,
        commitment: &str,
    ) -> Result<Option<TransactionInfo>> {
        self.call(
            "getTransaction",
            json!([
                signature,
                {
                    "encoding": "json",
                    "commitment": commitment,
                    "maxSupportedTransactionVersion": 0,
                }
            ]),
        )
        .await
    }
}
