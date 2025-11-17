//! Blockchain abstraction layer for Dungeon game.
//!
//! This crate provides a layered blockchain abstraction for the Dungeon game.
//!
//! # Architecture
//!
//! ```text
//! Layer 2: BlockchainClient (composite trait)
//!          ├── SessionManager
//!          ├── ProofSubmitter
//!          └── StateVerifier
//!
//! Layer 1: Domain Traits (game concepts)
//!
//! Layer 0: BlockchainTransport (pure infrastructure)
//! ```
//!
//! # Design Philosophy
//!
//! - **Layer 0 (Transport)**: Pure blockchain operations, no game knowledge
//! - **Layer 1 (Domain)**: Game-specific traits (sessions, proofs, state)
//! - **Layer 2 (Composite)**: Complete client interface combining all capabilities
//!
//! # Usage
//!
//! ```ignore
//! use client_blockchain_core::{BlockchainClient, SessionManager, ProofSubmitter, StateVerifier};
//!
//! // Use the high-level composite trait
//! async fn play_game(blockchain: &dyn BlockchainClient) {
//!     let session = blockchain.create_session(oracle_root, initial_state).await?;
//!     let receipt = blockchain.submit_proof(&session, proof).await?;
//!     blockchain.finalize_session(&session).await?;
//! }
//! ```

pub mod traits;
pub mod types;

#[cfg(test)]
pub mod mock;

// Re-export all traits
pub use traits::{
    BlockchainClient, BlockchainTransport, ProofError, ProofSubmitter, SessionError,
    SessionManager, StateError, StateVerifier, TransportError,
};

// Re-export all types
pub use types::{
    BlockchainConfig, GasEstimate, ObjectData, ObjectId, OnChainSession, ProofReceipt, SessionId,
    SessionStatus, StateRoot, TransactionData, TransactionId, TransactionStatus,
};

#[cfg(test)]
pub use mock::MockBlockchainClient;
