//! Blockchain abstraction traits.
//!
//! These traits define the interface for interacting with any blockchain backend.
//! Specific implementations (Sui, Ethereum, etc.) implement these traits.

use async_trait::async_trait;
use zk::ProofData;

use crate::types::{
    BlockchainConfig, ProofMetadata, SessionId, SessionState, SubmissionResult, TransactionId,
    TransactionStatus,
};

/// Errors that can occur during blockchain operations.
#[derive(Debug, thiserror::Error)]
pub enum BlockchainError {
    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Transaction failed: {0}")]
    TransactionFailed(String),

    #[error("Insufficient gas: required {required}, available {available}")]
    InsufficientGas { required: u64, available: u64 },

    #[error("Session not found: {0:?}")]
    SessionNotFound(SessionId),

    #[error("Invalid proof data: {0}")]
    InvalidProof(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Backend-specific error: {0}")]
    BackendError(String),
}

pub type Result<T> = std::result::Result<T, BlockchainError>;

/// Proof submission interface.
///
/// Handles conversion and submission of zkVM proofs to the blockchain.
#[async_trait]
pub trait ProofSubmitter: Send + Sync {
    /// Submit a single proof to the blockchain.
    ///
    /// # Arguments
    ///
    /// * `session_id` - On-chain game session identifier
    /// * `proof_data` - zkVM proof from `zk` crate
    ///
    /// # Returns
    ///
    /// Submission result with transaction ID and gas cost.
    async fn submit_proof(
        &self,
        session_id: &SessionId,
        proof_data: ProofData,
    ) -> Result<SubmissionResult>;

    /// Submit multiple proofs in a single transaction (batch).
    ///
    /// Reduces gas costs by amortizing transaction overhead.
    /// Some blockchains may not support batching, in which case this
    /// should fall back to sequential individual submissions.
    async fn submit_batch(
        &self,
        session_id: &SessionId,
        proofs: Vec<ProofData>,
    ) -> Result<Vec<SubmissionResult>>;

    /// Estimate gas cost for submitting a proof.
    ///
    /// Useful for user confirmation before submission.
    async fn estimate_gas(&self, session_id: &SessionId, proof_data: &ProofData) -> Result<u64>;

    /// Check transaction status.
    async fn check_transaction(&self, tx_id: &TransactionId) -> Result<TransactionStatus>;
}

/// Game session management on-chain.
///
/// Handles creation, querying, and finalization of game sessions.
#[async_trait]
pub trait SessionManager: Send + Sync {
    /// Create a new game session on-chain.
    ///
    /// # Arguments
    ///
    /// * `oracle_root` - Merkle root of static game content
    ///
    /// # Returns
    ///
    /// Session ID (object ID, contract address, etc.)
    async fn create_session(&self, oracle_root: [u8; 32]) -> Result<SessionId>;

    /// Query current session state from the blockchain.
    async fn get_session_state(&self, session_id: &SessionId) -> Result<SessionState>;

    /// Finalize a game session (mark as complete).
    ///
    /// After finalization, no more proofs can be submitted to this session.
    async fn finalize_session(&self, session_id: &SessionId) -> Result<TransactionId>;

    /// Check if a session exists and is active.
    async fn is_session_active(&self, session_id: &SessionId) -> Result<bool>;
}

/// High-level blockchain client interface.
///
/// Combines proof submission and session management with additional
/// convenience methods for managing pending proofs.
///
/// This is the main interface that client applications should use.
#[async_trait]
pub trait BlockchainClient: ProofSubmitter + SessionManager + Send + Sync {
    /// List all proofs pending submission (not yet on-chain).
    ///
    /// These proofs have been generated locally but not yet submitted.
    async fn list_pending_proofs(&self) -> Result<Vec<ProofMetadata>>;

    /// Submit all pending proofs for a session.
    ///
    /// This is a convenience method that:
    /// 1. Lists all pending proofs
    /// 2. Submits them in batches (if supported)
    /// 3. Returns aggregated results
    async fn submit_all_pending(&self, session_id: &SessionId) -> Result<Vec<SubmissionResult>>;

    /// Get the blockchain-specific configuration.
    fn config(&self) -> &dyn BlockchainConfig;

    /// Health check: verify connection to blockchain.
    async fn health_check(&self) -> Result<()>;
}
