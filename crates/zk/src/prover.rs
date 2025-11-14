//! Universal prover interface for zero-knowledge proof generation.
//!
//! Defines the common interface implemented by all proving backends.

use game_core::{Action, GameState};

/// ZK proof data container with journal and digest.
///
/// # RISC0 Groth16 Architecture
///
/// For RISC0 Groth16 proofs, the structure follows a two-stage verification model:
///
/// **Stage 1 (On-chain):** Groth16 proof verification
/// - `bytes`: Groth16 seal (proof)
/// - `journal_digest`: SHA-256(journal) - the actual Groth16 public input
///
/// **Stage 2 (On-chain):** Journal content verification
/// - `journal`: Raw journal bytes (168 bytes)
/// - Contract verifies: SHA-256(journal) == journal_digest
/// - Contract extracts 6 fields from journal
///
/// # Journal Structure (168 bytes)
///
/// ```text
/// 1. oracle_root       (32 bytes, offset 0..32)
/// 2. seed_commitment   (32 bytes, offset 32..64)
/// 3. prev_state_root   (32 bytes, offset 64..96)
/// 4. actions_root      (32 bytes, offset 96..128)
/// 5. new_state_root    (32 bytes, offset 128..160)
/// 6. new_nonce         (8 bytes, offset 160..168)
/// ```
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProofData {
    /// Proof bytes (Groth16 seal in production, or STARK receipt)
    pub bytes: Vec<u8>,

    /// Backend that generated this proof
    pub backend: ProofBackend,

    /// Raw journal bytes (all public outputs from guest program)
    ///
    /// For RISC0: 168 bytes containing 6 committed fields
    pub journal: Vec<u8>,

    /// SHA-256 digest of journal (the actual Groth16 public input)
    ///
    /// This is what gets verified in the Groth16 proof on-chain.
    pub journal_digest: [u8; 32],
}

/// Identifies which proving backend generated a proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ProofBackend {
    #[cfg(feature = "stub")]
    Stub,

    #[cfg(feature = "sp1")]
    Sp1,

    #[cfg(feature = "risc0")]
    Risc0,

    #[cfg(feature = "arkworks")]
    Arkworks,
}

/// Errors that can occur during proof generation or verification.
#[derive(Debug, thiserror::Error)]
pub enum ProofError {
    #[error("zkVM proof generation failed: {0}")]
    ZkvmError(String),

    #[cfg(feature = "arkworks")]
    #[error("Merkle tree construction failed: {0}")]
    MerkleTreeError(String),

    #[cfg(feature = "arkworks")]
    #[error("Witness generation failed: {0}")]
    WitnessError(String),

    #[cfg(feature = "arkworks")]
    #[error("Circuit proof generation failed: {0}")]
    CircuitProofError(String),

    #[error("State inconsistency: {0}")]
    StateInconsistency(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Invalid journal: {0}")]
    InvalidJournal(String),

    #[error("Journal digest mismatch: expected {expected:?}, got {actual:?}")]
    JournalDigestMismatch {
        expected: [u8; 32],
        actual: [u8; 32],
    },
}

// ============================================================================
// Journal Helper Functions
// ============================================================================

/// Journal field parsed from 168-byte journal.
///
/// Represents the 6 fields committed by the guest program.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JournalFields {
    pub oracle_root: [u8; 32],
    pub seed_commitment: [u8; 32],
    pub prev_state_root: [u8; 32],
    pub actions_root: [u8; 32],
    pub new_state_root: [u8; 32],
    pub new_nonce: u64,
}

/// Compute SHA-256 digest of journal bytes.
///
/// This is the public input to the Groth16 proof.
pub fn compute_journal_digest(journal: &[u8]) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    Sha256::digest(journal).into()
}

/// Parse journal bytes into individual fields.
///
/// # Journal Structure (168 bytes)
///
/// ```text
/// Offset  Size  Field
/// ------  ----  -----
/// 0..32   32    oracle_root
/// 32..64  32    seed_commitment
/// 64..96  32    prev_state_root
/// 96..128 32    actions_root
/// 128..160 32   new_state_root
/// 160..168 8    new_nonce (u64 little-endian)
/// ```
///
/// # Errors
///
/// Returns `ProofError::InvalidJournal` if journal size is not exactly 168 bytes.
pub fn parse_journal(journal: &[u8]) -> Result<JournalFields, ProofError> {
    if journal.len() != 168 {
        return Err(ProofError::InvalidJournal(format!(
            "Expected 168 bytes, got {}",
            journal.len()
        )));
    }

    Ok(JournalFields {
        oracle_root: journal[0..32].try_into().unwrap(),
        seed_commitment: journal[32..64].try_into().unwrap(),
        prev_state_root: journal[64..96].try_into().unwrap(),
        actions_root: journal[96..128].try_into().unwrap(),
        new_state_root: journal[128..160].try_into().unwrap(),
        new_nonce: u64::from_le_bytes(journal[160..168].try_into().unwrap()),
    })
}

/// Verify journal structure and compute digest.
///
/// Checks that:
/// 1. Journal is exactly 168 bytes
/// 2. Journal digest matches expected value (if provided)
///
/// Returns parsed journal fields.
pub fn verify_journal_structure(
    journal: &[u8],
    expected_digest: Option<&[u8; 32]>,
) -> Result<JournalFields, ProofError> {
    // Parse journal
    let fields = parse_journal(journal)?;

    // Verify digest if provided
    if let Some(expected) = expected_digest {
        let actual = compute_journal_digest(journal);
        if &actual != expected {
            return Err(ProofError::JournalDigestMismatch {
                expected: *expected,
                actual,
            });
        }
    }

    Ok(fields)
}

/// Universal prover interface for all proving backends.
///
/// All backends (zkVM, circuit, etc.) implement this trait to provide
/// a consistent API for proof generation and verification.
pub trait Prover: Send + Sync {
    /// Generate a zero-knowledge proof for action execution.
    ///
    /// Proves that executing `actions` on `start_state` produces `end_state`.
    ///
    /// Works for both single actions and batches:
    /// - Single action: `prove(state, &[action], expected_state)`
    /// - Batch: `prove(state, &[action1, action2, ...], expected_state)`
    ///
    /// The guest program will:
    /// 1. Start with `start_state`
    /// 2. Execute each action in `actions` sequentially
    /// 3. Compute intermediate states internally (not exposed to host)
    /// 4. Verify the final state matches `end_state`
    /// 5. Generate a single proof for the entire execution
    fn prove(
        &self,
        start_state: &GameState,
        actions: &[Action],
        end_state: &GameState,
    ) -> Result<ProofData, ProofError>;

    /// Verify a proof locally (for testing and debugging).
    ///
    /// Note: This is host-side verification. For on-chain verification,
    /// proofs must be submitted to a smart contract verifier.
    fn verify(&self, proof: &ProofData) -> Result<bool, ProofError>;
}

// ============================================================================
// Stub Prover
// ============================================================================

/// Stub prover for testing and development.
///
/// Returns dummy proofs without actual proof generation. Use for fast iteration
/// during development or testing without zkVM infrastructure.
///
/// **Warning**: Provides no cryptographic guarantees - do not use in production.
#[cfg(feature = "stub")]
#[derive(Debug, Clone)]
pub struct StubProver {
    #[allow(dead_code)]
    oracle_snapshot: crate::OracleSnapshot,
}

#[cfg(feature = "stub")]
impl StubProver {
    pub fn new(oracle_snapshot: crate::OracleSnapshot) -> Self {
        Self { oracle_snapshot }
    }
}

#[cfg(feature = "stub")]
impl Prover for StubProver {
    fn prove(
        &self,
        _start_state: &GameState,
        actions: &[Action],
        _end_state: &GameState,
    ) -> Result<ProofData, ProofError> {
        // Stub prover: return dummy proof with action count encoded
        let action_count = actions.len() as u32;
        let mut proof_bytes = vec![0x5A, 0x4B]; // "ZK" prefix
        proof_bytes.extend_from_slice(&action_count.to_le_bytes());
        proof_bytes.extend_from_slice(&[0xDE, 0xAD, 0xBE, 0xEF]);

        // Dummy journal (168 bytes of zeros)
        let journal = vec![0u8; 168];
        let journal_digest = compute_journal_digest(&journal);

        Ok(ProofData {
            bytes: proof_bytes,
            backend: ProofBackend::Stub,
            journal,
            journal_digest,
        })
    }

    fn verify(&self, proof: &ProofData) -> Result<bool, ProofError> {
        if proof.backend != ProofBackend::Stub {
            return Err(ProofError::ZkvmError(format!(
                "StubProver can only verify stub proofs, got {:?}",
                proof.backend
            )));
        }
        Ok(true)
    }
}
