//! Sui blockchain integration for Dungeon.
//!
//! ## Module Organization
//!
//! - [`client`]: Main `SuiBlockchainClient` facade
//! - [`config`]: Network configuration and environment loading
//! - [`contracts`]: Contract clients (GameSessionContract, etc.)
//! - `deployment`: Deployment info management
//! - `error`: Error types
//! - `utils`: Utilities (transaction building, type conversion)

pub mod client;
pub mod config;
pub mod contracts;
pub mod core;
pub mod utils;
pub mod walrus;

// Re-export primary types
pub use client::SuiBlockchainClient;
pub use config::{DeploymentInfo, SuiConfig, SuiNetwork};
pub use contracts::{GameSession, GameSessionContract};
pub use core::{ProofSubmission, Result, SessionId, StateRoot, SuiError, TxDigest};
pub use walrus::{BlobInfo, BlobObject, Network as WalrusNetwork, WalrusClient};
