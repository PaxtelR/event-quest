//! Shared environment/config loading for `apps/api` and `apps/indexer`.
//!
//! `AppConfig::from_env()` is the only supported way to construct
//! configuration. It enforces the mandatory startup gate from spec §12.1:
//! **the service refuses to start when `SOLANA_NETWORK` is not `devnet`.**
//! Mainnet support is out of scope for this project.

use std::env;
use std::str::FromStr;
use std::time::Duration;

use solana_pubkey::Pubkey;
use thiserror::Error;

/// Minimum/default/maximum QR rotation window, per spec §8.1. Intervals
/// below the minimum hurt camera readability, slow devices, and
/// accessibility; the program does not enforce this (it has no notion of a
/// QR interval), so the API is the only place this gets validated.
pub const QR_ROTATION_SECONDS_MIN: u64 = 10;
pub const QR_ROTATION_SECONDS_MAX: u64 = 60;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("missing required environment variable `{0}`")]
    Missing(&'static str),
    #[error("environment variable `{name}` has an invalid value: {reason}")]
    Invalid { name: &'static str, reason: String },
    #[error(
        "SOLANA_NETWORK must be `devnet` for this deployment; got `{0}`. \
         Mainnet is out of scope for EventQuest — see spec §0.7/§12.1."
    )]
    NotDevnet(String),
}

#[derive(Debug, Clone)]
pub struct SolanaConfig {
    pub network: String,
    pub rpc_http_url: String,
    pub rpc_ws_url: String,
    pub program_id: Pubkey,
    pub attestor_keypair_path: String,
}

#[derive(Debug, Clone)]
pub struct QrConfig {
    pub rotation_seconds: u64,
    pub grace_seconds: u64,
    pub checkin_grant_seconds: u64,
    pub signing_secret: String,
}

impl QrConfig {
    pub fn rotation(&self) -> Duration {
        Duration::from_secs(self.rotation_seconds)
    }

    pub fn grace(&self) -> Duration {
        Duration::from_secs(self.grace_seconds)
    }

    pub fn checkin_grant(&self) -> Duration {
        Duration::from_secs(self.checkin_grant_seconds)
    }
}

#[derive(Debug, Clone)]
pub struct SessionConfig {
    pub cookie_name: String,
    pub ttl_seconds: u64,
    pub auth_signing_secret: String,
}

impl SessionConfig {
    pub fn ttl(&self) -> Duration {
        Duration::from_secs(self.ttl_seconds)
    }
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub node_env: String,
    pub public_app_url: String,
    pub api_url: String,
    pub database_url: String,
    pub redis_url: String,
    pub solana: SolanaConfig,
    pub qr: QrConfig,
    pub session: SessionConfig,
    pub log_level: String,
    pub otel_exporter_otlp_endpoint: Option<String>,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        let network = optional("SOLANA_NETWORK", "devnet");
        if network != "devnet" {
            return Err(ConfigError::NotDevnet(network));
        }

        let program_id_raw = required("SOLANA_PROGRAM_ID")?;
        let program_id = Pubkey::from_str(&program_id_raw).map_err(|_| ConfigError::Invalid {
            name: "SOLANA_PROGRAM_ID",
            reason: format!("`{program_id_raw}` is not a valid base58 public key"),
        })?;

        let rotation_seconds = parse_u64(
            "QR_ROTATION_SECONDS",
            &optional("QR_ROTATION_SECONDS", "15"),
        )?;
        if !(QR_ROTATION_SECONDS_MIN..=QR_ROTATION_SECONDS_MAX).contains(&rotation_seconds) {
            return Err(ConfigError::Invalid {
                name: "QR_ROTATION_SECONDS",
                reason: format!(
                    "must be between {QR_ROTATION_SECONDS_MIN} and \
                     {QR_ROTATION_SECONDS_MAX} seconds, got {rotation_seconds}"
                ),
            });
        }

        Ok(Self {
            node_env: optional("NODE_ENV", "development"),
            public_app_url: required("PUBLIC_APP_URL")?,
            api_url: required("API_URL")?,
            database_url: required("DATABASE_URL")?,
            redis_url: required("REDIS_URL")?,
            solana: SolanaConfig {
                network,
                rpc_http_url: required("SOLANA_RPC_HTTP_URL")?,
                rpc_ws_url: required("SOLANA_RPC_WS_URL")?,
                program_id,
                attestor_keypair_path: required("ATTESTOR_KEYPAIR_PATH")?,
            },
            qr: QrConfig {
                rotation_seconds,
                grace_seconds: parse_u64("QR_GRACE_SECONDS", &optional("QR_GRACE_SECONDS", "5"))?,
                checkin_grant_seconds: parse_u64(
                    "CHECKIN_GRANT_SECONDS",
                    &optional("CHECKIN_GRANT_SECONDS", "30"),
                )?,
                signing_secret: required_secret("QR_SIGNING_SECRET")?,
            },
            session: SessionConfig {
                cookie_name: optional("SESSION_COOKIE_NAME", "eventquest_session"),
                ttl_seconds: parse_u64(
                    "SESSION_TTL_SECONDS",
                    &optional("SESSION_TTL_SECONDS", "86400"),
                )?,
                auth_signing_secret: required_secret("AUTH_SIGNING_SECRET")?,
            },
            log_level: optional("LOG_LEVEL", "info"),
            otel_exporter_otlp_endpoint: env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
                .ok()
                .filter(|s| !s.is_empty()),
        })
    }
}

fn required(name: &'static str) -> Result<String, ConfigError> {
    env::var(name)
        .ok()
        .filter(|v| !v.is_empty())
        .ok_or(ConfigError::Missing(name))
}

/// Like `required`, but for secrets — kept as a distinct function so it's
/// easy to grep for every place this service depends on a real secret being
/// configured (spec §19.1: never a hardcoded or default secret).
fn required_secret(name: &'static str) -> Result<String, ConfigError> {
    required(name)
}

fn optional(name: &str, default: &str) -> String {
    env::var(name)
        .ok()
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| default.to_string())
}

fn parse_u64(name: &'static str, value: &str) -> Result<u64, ConfigError> {
    value.parse().map_err(|_| ConfigError::Invalid {
        name,
        reason: format!("expected a non-negative integer, got `{value}`"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const REQUIRED_VARS: &[(&str, &str)] = &[
        ("PUBLIC_APP_URL", "http://localhost:3000"),
        ("API_URL", "http://localhost:3001"),
        (
            "DATABASE_URL",
            "postgresql://eventquest:eventquest@localhost:5432/eventquest",
        ),
        ("REDIS_URL", "redis://localhost:6379"),
        (
            "SOLANA_PROGRAM_ID",
            "CuULC7T6d49mf89sRPU8WLeYfkF74kDyEKvfCe2rXq8H",
        ),
        ("SOLANA_RPC_HTTP_URL", "https://api.devnet.solana.com"),
        ("SOLANA_RPC_WS_URL", "wss://api.devnet.solana.com"),
        ("ATTESTOR_KEYPAIR_PATH", ".secrets/attestor-devnet.json"),
        ("QR_SIGNING_SECRET", "test-qr-secret"),
        ("AUTH_SIGNING_SECRET", "test-auth-secret"),
    ];

    /// All env-var-dependent scenarios in one test function: `cargo test`
    /// runs tests in parallel threads within the same process, and
    /// `std::env::set_var` is process-global, so splitting these into
    /// separate `#[test]` functions would race.
    #[test]
    fn from_env_scenarios() {
        // SAFETY: single-threaded within this test function; no other test
        // in this crate touches these env vars.
        unsafe {
            for (key, _) in REQUIRED_VARS {
                env::remove_var(key);
            }
            env::remove_var("SOLANA_NETWORK");
            env::remove_var("QR_ROTATION_SECONDS");
        }

        // Missing everything -> missing SOLANA_PROGRAM_ID (the first
        // required check after the network gate, which defaults to devnet).
        assert!(matches!(
            AppConfig::from_env(),
            Err(ConfigError::Missing("SOLANA_PROGRAM_ID"))
        ));

        // SAFETY: see above.
        unsafe {
            for (key, value) in REQUIRED_VARS {
                env::set_var(key, value);
            }
        }

        // Defaults to devnet and succeeds once all required vars are set.
        let config = AppConfig::from_env().expect("config should load with all required vars");
        assert_eq!(config.solana.network, "devnet");
        assert_eq!(config.qr.rotation_seconds, 15);
        assert_eq!(
            config.solana.program_id.to_string(),
            "CuULC7T6d49mf89sRPU8WLeYfkF74kDyEKvfCe2rXq8H"
        );

        // Mainnet is explicitly rejected.
        // SAFETY: see above.
        unsafe {
            env::set_var("SOLANA_NETWORK", "mainnet-beta");
        }
        assert!(matches!(
            AppConfig::from_env(),
            Err(ConfigError::NotDevnet(network)) if network == "mainnet-beta"
        ));
        // SAFETY: see above.
        unsafe {
            env::set_var("SOLANA_NETWORK", "devnet");
        }

        // QR rotation below the safe minimum is rejected.
        // SAFETY: see above.
        unsafe {
            env::set_var("QR_ROTATION_SECONDS", "5");
        }
        assert!(matches!(
            AppConfig::from_env(),
            Err(ConfigError::Invalid {
                name: "QR_ROTATION_SECONDS",
                ..
            })
        ));

        // QR rotation above the safe maximum is rejected.
        // SAFETY: see above.
        unsafe {
            env::set_var("QR_ROTATION_SECONDS", "120");
        }
        assert!(matches!(
            AppConfig::from_env(),
            Err(ConfigError::Invalid {
                name: "QR_ROTATION_SECONDS",
                ..
            })
        ));

        // Invalid program id is rejected with a clear error.
        // SAFETY: see above.
        unsafe {
            env::set_var("QR_ROTATION_SECONDS", "15");
            env::set_var("SOLANA_PROGRAM_ID", "not-a-valid-pubkey");
        }
        assert!(matches!(
            AppConfig::from_env(),
            Err(ConfigError::Invalid {
                name: "SOLANA_PROGRAM_ID",
                ..
            })
        ));

        // Clean up so other tests in the same binary aren't affected.
        // SAFETY: see above.
        unsafe {
            for (key, _) in REQUIRED_VARS {
                env::remove_var(key);
            }
            env::remove_var("SOLANA_NETWORK");
            env::remove_var("QR_ROTATION_SECONDS");
        }
    }
}
