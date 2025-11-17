//! Blockchain abstraction layer for Dungeon game.
//!
//! This crate provides blockchain-agnostic interfaces for proof submission
//! and game session management. Specific blockchain implementations (Sui, Ethereum)
//! are in separate crates that implement these traits.
//!
//! # Architecture
//!
//! ```text
//! runtime (ProofEvent)
//!     ↓
//! client-blockchain-core (traits)
//!     ↓
//! ├── client-blockchain-sui (Sui implementation)
//! ├── client-blockchain-ethereum (Ethereum implementation)
//! └── client-blockchain-starknet (StarkNet implementation)
//! ```
//!
//! # Design Philosophy
//!
//! Following the same pattern as `zk::Prover`:
//! - Define common interfaces as traits
//! - Specific implementations in separate crates
//! - Feature-gated compilation for each backend
//! - Mock implementation for testing
//!
//! # Usage
//!
//! ```ignore
//! use client_blockchain_core::{BlockchainClient, ProofSubmitter};
//!
//! // Load backend-specific client
//! #[cfg(feature = "sui")]
//! let client = client_blockchain_sui::SuiBlockchainClient::new(config).await?;
//!
//! // Use blockchain-agnostic interface
//! let session_id = client.create_session(oracle_root).await?;
//! let tx_id = client.submit_proof(session_id, proof_data).await?;
//! ```

pub mod traits;
pub mod types;

#[cfg(test)]
pub mod mock;

pub use traits::{BlockchainClient, BlockchainError, ProofSubmitter, Result, SessionManager};
pub use types::{
    BlockchainConfig, ProofMetadata, SessionId, SessionState, SubmissionResult, TransactionId,
    TransactionStatus,
};

#[cfg(test)]
pub use mock::MockBlockchainClient;
