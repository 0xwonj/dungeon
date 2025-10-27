//! Proof index for tracking proof generation progress.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::types::{ByteOffset, DurationMs, Nonce, ProofSize, SessionId, Timestamp};

/// Index tracking proof generation progress for a session.
///
/// # Data Layout
///
/// proof_index_{session}.json  ← This structure (lightweight metadata)
/// proofs/                     ← Actual proof files
///   ├── proof_0.bin
///   ├── proof_1.bin
///   └── proof_2.bin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofIndex {
    /// Session this proof index belongs to
    pub session_id: SessionId,

    /// Highest nonce with completed and verified proof
    ///
    /// All proofs for nonce 0..=proven_up_to_nonce are complete.
    /// This is the safe resume point for proof generation.
    pub proven_up_to_nonce: Nonce,

    /// Total number of proofs generated
    pub total_proofs: u64,

    /// Last update timestamp
    pub updated_at: Timestamp,

    /// Action log byte offset after processing proven_up_to_nonce
    ///
    /// This is the byte position in the action log file where ProverWorker
    /// should resume reading. Allows efficient checkpoint/resume without
    /// scanning from the beginning of the file.
    #[serde(default)]
    pub action_log_offset: ByteOffset,

    /// Individual proof entries (sparse, only stores completed proofs)
    ///
    /// Key: nonce
    /// Value: ProofEntry metadata
    ///
    /// This is sparse because we might checkpoint every 10 actions
    /// but only store proof metadata for checkpointed states.
    #[serde(default)]
    pub proofs: BTreeMap<Nonce, ProofEntry>,
}

/// Metadata for a single completed proof.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofEntry {
    /// Action nonce this proof corresponds to
    pub nonce: Nonce,

    /// When this proof was generated
    pub timestamp: Timestamp,

    /// Time taken to generate this proof (milliseconds)
    pub generation_time_ms: DurationMs,

    /// Whether proof file is persisted to disk
    pub is_persisted: bool,

    /// Whether proof has been verified
    pub is_verified: bool,

    /// Optional path to proof file (relative to proofs directory)
    pub proof_file: Option<String>,

    /// Size of proof in bytes (if persisted)
    pub proof_size_bytes: Option<ProofSize>,
}

impl ProofIndex {
    /// Create a new empty proof index.
    pub fn new(session_id: String) -> Self {
        Self {
            session_id,
            proven_up_to_nonce: 0,
            total_proofs: 0,
            updated_at: current_timestamp(),
            action_log_offset: 0,
            proofs: BTreeMap::new(),
        }
    }

    /// Record a newly completed proof.
    ///
    /// Updates proven_up_to_nonce if this proof fills a gap.
    pub fn add_proof(&mut self, entry: ProofEntry) {
        let nonce = entry.nonce;
        self.proofs.insert(nonce, entry);
        self.total_proofs += 1;
        self.updated_at = current_timestamp();

        // Update proven_up_to_nonce if we filled a gap
        self.update_proven_up_to();
    }

    /// Update proven_up_to_nonce to reflect contiguous proof coverage.
    ///
    /// proven_up_to_nonce is the highest nonce where all proofs 0..=nonce exist.
    fn update_proven_up_to(&mut self) {
        let mut contiguous_nonce = 0;

        // Find highest contiguous nonce
        for nonce in 0.. {
            if self.proofs.contains_key(&nonce) {
                contiguous_nonce = nonce;
            } else {
                break;
            }
        }

        self.proven_up_to_nonce = contiguous_nonce;
    }

    /// Check if a proof exists for a given nonce.
    pub fn has_proof(&self, nonce: Nonce) -> bool {
        self.proofs.contains_key(&nonce)
    }

    /// Get proof entry for a specific nonce.
    pub fn get_proof(&self, nonce: Nonce) -> Option<&ProofEntry> {
        self.proofs.get(&nonce)
    }

    /// Get the gap between game progress and proof progress.
    ///
    /// Returns the number of actions that need proving.
    pub fn gap_from_game_nonce(&self, game_nonce: Nonce) -> Nonce {
        game_nonce.saturating_sub(self.proven_up_to_nonce)
    }

    /// List all proven nonces in order.
    pub fn proven_nonces(&self) -> Vec<Nonce> {
        self.proofs.keys().copied().collect()
    }

    /// Check if all proofs up to a nonce are complete (contiguous).
    pub fn is_proven_up_to(&self, nonce: Nonce) -> bool {
        nonce <= self.proven_up_to_nonce
    }
}

impl ProofEntry {
    /// Create a new proof entry.
    pub fn new(nonce: Nonce, generation_time_ms: DurationMs) -> Self {
        Self {
            nonce,
            timestamp: current_timestamp(),
            generation_time_ms,
            is_persisted: false,
            is_verified: true, // zkVM proofs are verified during generation
            proof_file: None,
            proof_size_bytes: None,
        }
    }

    /// Mark this proof as persisted with file information.
    pub fn with_file(mut self, filename: String, size_bytes: ProofSize) -> Self {
        self.is_persisted = true;
        self.proof_file = Some(filename);
        self.proof_size_bytes = Some(size_bytes);
        self
    }
}

/// Get current unix timestamp in seconds.
fn current_timestamp() -> Timestamp {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}
