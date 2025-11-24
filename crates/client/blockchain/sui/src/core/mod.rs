//! Core types and errors for Sui blockchain integration.

pub mod error;
pub mod types;

// Re-export commonly used items
pub use error::{Result, SuiError};
pub use types::{ProofSubmission, SessionId, StateRoot, TxDigest};
