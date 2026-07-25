//! Solana RPC/transaction integration: building instructions via
//! `crates/eventquest-chain`'s generated client, co-signing as the
//! checkpoint attestor, submitting, and confirming — spec §9.3/§9.5.
//!
//! Deliberately hand-rolls the (small) JSON-RPC surface it needs
//! (`rpc.rs`) instead of depending on `solana-rpc-client` — see this
//! crate's Cargo.toml for why that crate can't be used here without
//! reintroducing the exact dependency conflict this module works around.

mod client;
mod rpc;

pub use client::{parse_pubkey, BlockchainClient};
