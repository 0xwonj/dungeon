//! Error types for Sui blockchain operations.

use thiserror::Error;

/// Errors that can occur during Sui blockchain operations.
#[derive(Debug, Error)]
pub enum SuiError {
    #[error("Network error: {0}")]
    Network(String),

    #[error("Transaction failed: {0}")]
    TransactionFailed(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("Session is finalized and cannot be modified")]
    SessionFinalized,

    #[error("Proof verification failed: {0}")]
    ProofVerificationFailed(String),

    #[error("Invalid proof data: {0}")]
    InvalidProof(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Object not found: {0}")]
    ObjectNotFound(String),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, SuiError>;
