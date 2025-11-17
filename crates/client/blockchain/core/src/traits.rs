//! Blockchain abstraction traits.
//!
//! This module defines a layered blockchain abstraction:
//! - Layer 0: BlockchainTransport (pure infrastructure)
//! - Layer 1: SessionManager, ProofSubmitter, StateVerifier (game domain)
//! - Layer 2: GameBlockchain (composite trait)

use async_trait::async_trait;
use zk::ProofData;

use crate::types::{
    GasEstimate, ObjectData, ObjectId, OnChainSession, ProofReceipt, SessionId, StateRoot,
    TransactionData, TransactionId, TransactionStatus,
};

// ============================================================================
// Error Types
// ============================================================================

/// Transport layer errors.
#[derive(Debug, thiserror::Error)]
pub enum TransportError {
    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Transaction failed: {0}")]
    TransactionFailed(String),

    #[error("Insufficient gas: required {required}, available {available}")]
    InsufficientGas { required: u64, available: u64 },

    #[error("Object not found: {0:?}")]
    ObjectNotFound(ObjectId),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Backend-specific error: {0}")]
    BackendError(String),
}

/// Session management errors.
#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error("Session not found: {0:?}")]
    SessionNotFound(SessionId),

    #[error("Session is not active")]
    SessionInactive,

    #[error("Session already finalized")]
    AlreadyFinalized,

    #[error("Transport error: {0}")]
    TransportError(#[from] TransportError),

    #[error("Invalid session data: {0}")]
    InvalidData(String),
}

/// Proof submission errors.
#[derive(Debug, thiserror::Error)]
pub enum ProofError {
    #[error("Invalid proof data: {0}")]
    InvalidProof(String),

    #[error("Proof verification failed: {0}")]
    VerificationFailed(String),

    #[error("Session not found: {0:?}")]
    SessionNotFound(SessionId),

    #[error("Session is not active")]
    SessionInactive,

    #[error("Proof already submitted for nonce {0}")]
    DuplicateProof(u64),

    #[error("Transport error: {0}")]
    TransportError(#[from] TransportError),

    #[error("Encoding error: {0}")]
    EncodingError(String),
}

/// State verification errors.
#[derive(Debug, thiserror::Error)]
pub enum StateError {
    #[error("Session not found: {0:?}")]
    SessionNotFound(SessionId),

    #[error("State inconsistency detected: on-chain {on_chain:?}, local {local:?}")]
    Inconsistency {
        on_chain: StateRoot,
        local: StateRoot,
    },

    #[error("Transport error: {0}")]
    TransportError(#[from] TransportError),

    #[error("Invalid state data: {0}")]
    InvalidData(String),
}

// ============================================================================
// Layer 0: Pure Infrastructure
// ============================================================================

/// Pure blockchain infrastructure layer.
///
/// This trait provides low-level blockchain operations without any game-specific knowledge.
#[async_trait]
pub trait BlockchainTransport: Send + Sync {
    /// Submit a transaction to the blockchain.
    async fn submit_transaction(&self, tx_data: TransactionData) -> Result<TransactionId, TransportError>;

    /// Query transaction status.
    async fn query_transaction(&self, tx_id: &TransactionId) -> Result<TransactionStatus, TransportError>;

    /// Query on-chain object/state.
    async fn query_object(&self, object_id: &ObjectId) -> Result<ObjectData, TransportError>;

    /// Estimate gas for a transaction.
    async fn estimate_gas(&self, tx_data: &TransactionData) -> Result<u64, TransportError>;

    /// Health check: verify connection to blockchain.
    async fn health_check(&self) -> Result<(), TransportError>;
}

// ============================================================================
// Layer 1: Game Domain Traits
// ============================================================================

/// Session lifecycle management.
///
/// Handles creation, querying, and finalization of game sessions on-chain.
#[async_trait]
pub trait SessionManager: Send + Sync {
    /// Create a new game session on-chain.
    async fn create_session(
        &self,
        oracle_root: [u8; 32],
        initial_state_root: [u8; 32],
    ) -> Result<SessionId, SessionError>;

    /// Query current session state from blockchain.
    async fn get_session(&self, session_id: &SessionId) -> Result<OnChainSession, SessionError>;

    /// Finalize a session (mark as complete).
    async fn finalize_session(&self, session_id: &SessionId) -> Result<TransactionId, SessionError>;

    /// Check if a session is active and accepting proofs.
    async fn is_session_active(&self, session_id: &SessionId) -> Result<bool, SessionError>;
}

/// ZK proof submission interface.
///
/// Handles conversion and submission of zkVM proofs to the blockchain.
#[async_trait]
pub trait ProofSubmitter: Send + Sync {
    /// Submit a single ZK proof to the blockchain.
    async fn submit_proof(
        &self,
        session_id: &SessionId,
        proof: ProofData,
    ) -> Result<ProofReceipt, ProofError>;

    /// Estimate gas cost for submitting a proof.
    async fn estimate_proof_gas(
        &self,
        session_id: &SessionId,
        proof: &ProofData,
    ) -> Result<GasEstimate, ProofError>;

    /// Submit multiple proofs in a batch (default: sequential submission).
    async fn submit_proof_batch(
        &self,
        session_id: &SessionId,
        proofs: Vec<ProofData>,
    ) -> Result<Vec<ProofReceipt>, ProofError> {
        let mut receipts = Vec::new();
        for proof in proofs {
            let receipt = self.submit_proof(session_id, proof).await?;
            receipts.push(receipt);
        }
        Ok(receipts)
    }
}

/// State verification and synchronization.
///
/// Provides methods for verifying local state matches on-chain verified state.
#[async_trait]
pub trait StateVerifier: Send + Sync {
    /// Get the latest verified state root from blockchain.
    async fn get_verified_state_root(&self, session_id: &SessionId) -> Result<StateRoot, StateError>;

    /// Get the nonce (action counter) for a session.
    async fn get_session_nonce(&self, session_id: &SessionId) -> Result<u64, StateError>;

    /// Verify that local state matches on-chain verified state.
    async fn verify_state_consistency(
        &self,
        session_id: &SessionId,
        local_state_root: StateRoot,
    ) -> Result<bool, StateError> {
        let on_chain_state = self.get_verified_state_root(session_id).await?;
        Ok(local_state_root == on_chain_state)
    }
}

// ============================================================================
// Layer 2: Composite Trait
// ============================================================================

/// Core game blockchain operations.
///
/// All game-compatible blockchains must implement this trait.
/// This is a composite of required domain traits.
pub trait GameBlockchain: SessionManager + ProofSubmitter + StateVerifier + Send + Sync {
    /// Get the blockchain name (e.g., "Sui", "Ethereum").
    fn name(&self) -> &str;

    /// Get the network name (e.g., "mainnet", "testnet", "local").
    fn network(&self) -> &str;
}
