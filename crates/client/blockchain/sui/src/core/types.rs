//! Common types for Sui blockchain contracts.

use serde::{Deserialize, Serialize};

use super::error::{Result, SuiError};

// ============================================================================
// Identifiers
// ============================================================================

/// Session identifier (Sui ObjectID).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(pub String);

impl SessionId {
    pub fn new(object_id: String) -> Self {
        Self(object_id)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Transaction digest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TxDigest(pub String);

impl TxDigest {
    pub fn new(digest: String) -> Self {
        Self(digest)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// State root type.
pub type StateRoot = [u8; 32];

// ============================================================================
// Proof Submission
// ============================================================================

/// Data needed to submit a proof and update session.
#[derive(Debug, Clone)]
pub struct ProofSubmission {
    /// Proof in arkworks format
    pub proof_points: Vec<u8>,

    /// SHA-256 digest of journal (public input)
    pub journal_digest: [u8; 32],

    /// Full journal bytes (168 bytes)
    pub journal: Vec<u8>,

    /// Serialized action log (uploaded to Walrus before submission)
    pub action_log: Vec<u8>,
}

impl ProofSubmission {
    /// Create proof submission from ZK proof data and action log.
    ///
    /// # Arguments
    ///
    /// * `proof` - ZK proof data (SP1/RISC0)
    /// * `action_log` - Serialized action sequence (will be uploaded to Walrus)
    ///
    /// # Returns
    ///
    /// ProofSubmission ready for blockchain submission
    pub fn from_proof_data(proof: &zk::ProofData, action_log: Vec<u8>) -> Result<Self> {
        // Convert proof to Sui format and extract public inputs (journal)
        let (proof_points, journal) = convert_sp1_proof_to_sui(proof)?;

        Ok(Self {
            proof_points,
            journal_digest: proof.journal_digest,
            journal,
            action_log,
        })
    }

    /// Parse journal to extract new state values.
    pub fn parse_journal(&self) -> Result<(StateRoot, u64)> {
        let fields =
            zk::parse_journal(&self.journal).map_err(|e| SuiError::InvalidProof(e.to_string()))?;

        Ok((fields.new_state_root, fields.new_nonce))
    }
}

// ============================================================================
// SP1 Proof Conversion
// ============================================================================

/// Convert SP1 proof to Sui arkworks format and extract journal.
///
/// Returns (proof_points, journal) where:
/// - proof_points: Arkworks-formatted proof for Sui verification
/// - journal: 168-byte public inputs/values
#[cfg(feature = "sp1")]
fn convert_sp1_proof_to_sui(proof: &zk::ProofData) -> Result<(Vec<u8>, Vec<u8>)> {
    use sp1_sdk::SP1ProofWithPublicValues;
    use sp1_sui::convert_sp1_gnark_to_ark;

    if !matches!(proof.backend, zk::ProofBackend::Sp1) {
        return Err(SuiError::InvalidProof(format!(
            "Expected SP1 proof, got {:?}",
            proof.backend
        )));
    }

    let sp1_proof: SP1ProofWithPublicValues =
        bincode::deserialize(&proof.bytes).map_err(|e| SuiError::Serialization(e.to_string()))?;

    // Extract journal (public values) BEFORE conversion
    // This must be done before convert_sp1_gnark_to_ark consumes sp1_proof
    let journal = sp1_proof.public_values.to_vec();

    // Validate journal size
    if journal.len() != 168 {
        return Err(SuiError::InvalidProof(format!(
            "Invalid journal size: expected 168 bytes, got {} bytes. \
             SP1 Groth16 proof does not contain valid public values.",
            journal.len()
        )));
    }

    // Convert to arkworks format for Sui
    let (_vk, _public_inputs, proof_points) = convert_sp1_gnark_to_ark(sp1_proof);

    Ok((proof_points, journal))
}

#[cfg(not(feature = "sp1"))]
fn convert_sp1_proof_to_sui(_proof: &zk::ProofData) -> Result<(Vec<u8>, Vec<u8>)> {
    Err(SuiError::InvalidProof(
        "SP1 feature not enabled".to_string(),
    ))
}
