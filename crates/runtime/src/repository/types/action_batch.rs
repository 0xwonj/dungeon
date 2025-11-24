//! Action batch data structure for checkpoint-based action grouping.
//!
//! An action batch represents a logical group of actions bounded by checkpoints.
//! Each batch:
//! - Has a single action log file
//! - Will be proven together as a single ZK proof
//! - Maps to one on-chain submission
//!
//! # Lifecycle
//!
//! ```text
//! InProgress → Complete → Proving → Proven → UploadingToWalrus → BlobUploaded → SubmittingOnchain → OnChain
//!     ↓           ↓          ↓         ↓              ↓                ↓                ↓              ↓
//! PersistenceW   CP      ProverW   ProverW      OnchainW         OnchainW         OnchainW       OnchainW
//! ```
//!
//! # File Path Convention
//!
//! ActionBatch does not store file paths. Instead, canonical paths are computed:
//! - Action log: `{base_dir}/{session_id}/actions/actions_{start:010}.bin`
//! - Proof file: `{base_dir}/{session_id}/proofs/proof_{start:010}.bin`
//!
//! # Replaces ProofIndex
//!
//! This type replaces the old ProofIndex for tracking proof status. All proof-related
//! information is now stored in the ActionBatchStatus::Proven variant.

use serde::{Deserialize, Serialize};

use crate::types::{Nonce, SessionId};

/// Represents a batch of actions bounded by checkpoints.
///
/// An action batch is the fundamental unit for:
/// - Action log file rotation (one file per batch)
/// - ZK proof generation (one proof per batch)
/// - On-chain submission (one transaction per batch)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionBatch {
    /// Session ID this batch belongs to
    pub session_id: SessionId,

    /// Starting nonce of this batch (inclusive)
    pub start_nonce: Nonce,

    /// Ending nonce of this batch (inclusive)
    pub end_nonce: Nonce,

    /// Current status of this batch
    pub status: ActionBatchStatus,
}

/// Status of an action batch through its lifecycle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionBatchStatus {
    /// Actions are being written to the log file (current batch)
    InProgress,

    /// Action log is complete (checkpoint created, file closed)
    Complete,

    /// ZK proof generation is in progress
    Proving,

    /// ZK proof generation completed successfully
    Proven {
        /// Proof file name (e.g., "proof_0000000000_0000000009.bin")
        proof_file: String,
        /// Proof generation time in milliseconds
        generation_time_ms: u64,
    },

    /// Uploading action log to Walrus (for on-chain verification)
    UploadingToWalrus,

    /// Action log successfully uploaded to Walrus
    BlobUploaded {
        /// Sui object ID of the Blob (for Move contract calls)
        blob_object_id: String,
        /// Walrus blob ID (base64, for HTTP retrieval)
        walrus_blob_id: String,
    },

    /// Submitting proof + blob to on-chain contract
    SubmittingOnchain {
        /// Sui object ID of the Blob (for reference)
        blob_object_id: String,
        /// Walrus blob ID (for reference)
        walrus_blob_id: String,
    },

    /// Successfully submitted to on-chain contract
    OnChain {
        /// Sui object ID of the Blob (for reference)
        blob_object_id: String,
        /// Walrus blob ID (for reference)
        walrus_blob_id: String,
        /// Transaction digest on Sui
        tx_digest: String,
    },

    /// Failed at some stage (can be retried)
    Failed {
        /// Error message
        error: String,
        /// Number of retry attempts
        retry_count: u32,
    },
}

impl ActionBatch {
    /// Create a new action batch starting at the given nonce.
    ///
    /// The batch is initially in `InProgress` status.
    pub fn new(session_id: SessionId, start_nonce: Nonce) -> Self {
        Self {
            session_id,
            start_nonce,
            end_nonce: start_nonce, // Will be updated as actions are added
            status: ActionBatchStatus::InProgress,
        }
    }

    /// Get the action log filename for this batch.
    ///
    /// Format: `actions_{start_nonce:010}.bin`
    pub fn action_log_filename(&self) -> String {
        format!("actions_{:010}.bin", self.start_nonce)
    }

    /// Get the batch metadata filename.
    ///
    /// Format: `batch_{start_nonce:010}.json`
    ///
    /// Only start_nonce is used to ensure the filename is known when the batch is created.
    pub fn batch_filename(&self) -> String {
        format!("batch_{:010}.json", self.start_nonce)
    }

    /// Get the proof filename for this batch.
    ///
    /// Format: `proof_{start_nonce:010}_{end_nonce:010}.bin`
    pub fn proof_filename(&self) -> String {
        format!("proof_{:010}_{:010}.bin", self.start_nonce, self.end_nonce)
    }

    /// Check if a nonce is within this batch's range.
    pub fn contains_nonce(&self, nonce: Nonce) -> bool {
        nonce >= self.start_nonce && nonce <= self.end_nonce
    }

    /// Get the number of actions in this batch.
    pub fn action_count(&self) -> u64 {
        if self.end_nonce >= self.start_nonce {
            self.end_nonce - self.start_nonce + 1
        } else {
            0
        }
    }

    /// Mark this batch as complete (checkpoint created).
    pub fn mark_complete(&mut self, end_nonce: Nonce) {
        self.end_nonce = end_nonce;
        self.status = ActionBatchStatus::Complete;
    }

    /// Mark this batch as being proven.
    pub fn mark_proving(&mut self) {
        self.status = ActionBatchStatus::Proving;
    }

    /// Mark this batch as proven.
    pub fn mark_proven(&mut self, proof_file: String, generation_time_ms: u64) {
        self.status = ActionBatchStatus::Proven {
            proof_file,
            generation_time_ms,
        };
    }

    /// Mark this batch as failed.
    pub fn mark_failed(&mut self, error: String, retry_count: u32) {
        self.status = ActionBatchStatus::Failed { error, retry_count };
    }

    /// Update the status.
    pub fn update_status(&mut self, status: ActionBatchStatus) {
        self.status = status;
    }

    /// Check if this batch is ready for proving (Complete status).
    pub fn is_ready_for_proving(&self) -> bool {
        matches!(self.status, ActionBatchStatus::Complete)
    }

    /// Check if this batch has a proof (Proven status).
    pub fn has_proof(&self) -> bool {
        matches!(self.status, ActionBatchStatus::Proven { .. })
    }

    /// Check if this batch is on-chain.
    pub fn is_onchain(&self) -> bool {
        matches!(self.status, ActionBatchStatus::OnChain { .. })
    }

    /// Get proof metadata if available.
    pub fn get_proof_info(&self) -> Option<(&str, u64)> {
        match &self.status {
            ActionBatchStatus::Proven {
                proof_file,
                generation_time_ms,
            } => Some((proof_file.as_str(), *generation_time_ms)),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_batch() {
        let batch = ActionBatch::new("session123".to_string(), 0);
        assert_eq!(batch.start_nonce, 0);
        assert_eq!(batch.end_nonce, 0);
        assert!(matches!(batch.status, ActionBatchStatus::InProgress));
    }

    #[test]
    fn test_action_log_filename() {
        let batch = ActionBatch::new("session123".to_string(), 0);
        assert_eq!(batch.action_log_filename(), "actions_0000000000.bin");

        let batch2 = ActionBatch::new("session123".to_string(), 42);
        assert_eq!(batch2.action_log_filename(), "actions_0000000042.bin");
    }

    #[test]
    fn test_proof_filename() {
        let mut batch = ActionBatch::new("session123".to_string(), 0);
        batch.end_nonce = 9;
        assert_eq!(batch.proof_filename(), "proof_0000000000_0000000009.bin");
    }

    #[test]
    fn test_contains_nonce() {
        let mut batch = ActionBatch::new("session123".to_string(), 10);
        batch.end_nonce = 19;

        assert!(!batch.contains_nonce(9));
        assert!(batch.contains_nonce(10));
        assert!(batch.contains_nonce(15));
        assert!(batch.contains_nonce(19));
        assert!(!batch.contains_nonce(20));
    }

    #[test]
    fn test_action_count() {
        let mut batch = ActionBatch::new("session123".to_string(), 0);
        batch.end_nonce = 9;
        assert_eq!(batch.action_count(), 10);

        let mut batch2 = ActionBatch::new("session123".to_string(), 10);
        batch2.end_nonce = 19;
        assert_eq!(batch2.action_count(), 10);
    }

    #[test]
    fn test_status_transitions() {
        let mut batch = ActionBatch::new("session123".to_string(), 0);

        batch.mark_complete(9);
        assert!(matches!(batch.status, ActionBatchStatus::Complete));
        assert_eq!(batch.end_nonce, 9);

        batch.mark_proving();
        assert!(matches!(batch.status, ActionBatchStatus::Proving));

        batch.mark_proven("proof_file.bin".to_string(), 1000);
        assert!(batch.has_proof());

        if let Some((file, time)) = batch.get_proof_info() {
            assert_eq!(file, "proof_file.bin");
            assert_eq!(time, 1000);
        } else {
            panic!("Expected proof info");
        }
    }
}
