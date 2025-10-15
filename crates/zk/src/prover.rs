//! Universal prover interface for zero-knowledge proof generation.
//!
//! Defines the common interface implemented by all proving backends.

use game_core::{Action, GameState, StateDelta};

/// ZK proof data container.
///
/// Contains serialized proof bytes and backend identifier.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProofData {
    pub bytes: Vec<u8>,
    pub backend: ProofBackend,
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
}

/// Universal prover interface for all proving backends.
///
/// All backends (zkVM, circuit, etc.) implement this trait to provide
/// a consistent API for proof generation and verification.
pub trait Prover: Send + Sync {
    /// Generate a zero-knowledge proof for an action execution.
    ///
    /// Proves that executing `action` on `before_state` produces `after_state`.
    fn prove(
        &self,
        before_state: &GameState,
        action: &Action,
        after_state: &GameState,
        delta: &StateDelta,
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
#[derive(Debug, Clone, Copy, Default)]
pub struct StubProver;

#[cfg(feature = "stub")]
impl StubProver {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(feature = "stub")]
impl Prover for StubProver {
    fn prove(
        &self,
        _before_state: &GameState,
        _action: &Action,
        _after_state: &GameState,
        _delta: &StateDelta,
    ) -> Result<ProofData, ProofError> {
        Ok(ProofData {
            bytes: vec![0xDE, 0xAD, 0xBE, 0xEF],
            backend: ProofBackend::Stub,
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
