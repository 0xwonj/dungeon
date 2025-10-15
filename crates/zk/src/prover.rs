//! Universal prover interface for zero-knowledge proof generation.
//!
//! This module defines:
//! - `Prover` trait: Common interface for all proving backends
//! - `ProofData`: Proof result container
//! - `ProofBackend`: Backend identifier enum
//! - `ProofError`: Unified error type for proof generation

use game_core::{Action, GameState, StateDelta};

/// ZK proof data container.
///
/// Contains the serialized proof bytes and identifies which backend generated it.
/// The internal proof format depends on the backend:
/// - **RISC0/SP1**: zkVM receipt with execution trace
/// - **Arkworks**: Groth16 proof (future)
/// - **Stub**: Dummy bytes for testing
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProofData {
    /// Proof bytes (format depends on backend)
    pub bytes: Vec<u8>,

    /// Backend identifier
    pub backend: ProofBackend,
}

/// Identifies which proving backend generated a proof.
///
/// Used for routing verification and understanding proof format.
/// Each variant is only available when the corresponding feature is enabled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ProofBackend {
    /// No proof generated (game-only mode)
    None,

    /// Stub prover (used for fast development/testing)
    #[cfg(feature = "std")]
    Stub,

    /// SP1 zkVM backend
    #[cfg(feature = "sp1")]
    Sp1,

    /// RISC0 zkVM backend
    #[cfg(feature = "risc0")]
    Risc0,

    /// Arkworks circuit backend (future)
    #[cfg(feature = "arkworks")]
    Arkworks,
}

/// Errors that can occur during proof generation or verification.
///
/// This error type covers failures from all proving backends.
/// Backend-specific errors are wrapped in the appropriate variant.
#[derive(Debug, thiserror::Error)]
pub enum ProofError {
    /// zkVM proof generation failed
    ///
    /// Covers errors from RISC0, SP1, or stub provers.
    #[error("zkVM proof generation failed: {0}")]
    ZkvmError(String),

    /// Merkle tree construction failed
    #[cfg(feature = "arkworks")]
    #[error("Merkle tree construction failed: {0}")]
    MerkleTreeError(String),

    /// Witness generation failed
    #[cfg(feature = "arkworks")]
    #[error("Witness generation failed: {0}")]
    WitnessError(String),

    /// Circuit proof generation failed
    #[cfg(feature = "arkworks")]
    #[error("Circuit proof generation failed: {0}")]
    CircuitProofError(String),

    /// State inconsistency detected
    ///
    /// The prover detected that the state transition is invalid.
    #[error("State inconsistency: {0}")]
    StateInconsistency(String),

    /// Serialization or deserialization error
    ///
    /// Failed to serialize/deserialize proof data or inputs.
    #[error("Serialization error: {0}")]
    SerializationError(String),
}

/// Universal prover interface for all proving backends.
///
/// This trait abstracts over different proving systems:
/// - **zkVM backends**: RISC0, SP1, Stub
/// - **Circuit provers**: Arkworks, other SNARK/STARK systems
/// - **Hybrid approaches**: Future proving mechanisms
///
/// All provers implement the same interface, allowing the runtime to be
/// completely agnostic to the underlying proving technology.
///
/// # Design Philosophy
///
/// The trait is intentionally minimal - only the operations that every prover
/// must support. Backend-specific functionality should be exposed through
/// concrete types, not through this trait.
///
/// # Example
///
/// ```rust,ignore
/// use zk::{Prover, ZkProver};
///
/// let prover = ZkProver::new(oracle_snapshot);
/// let proof = prover.prove(&before, &action, &after, &delta)?;
/// let is_valid = prover.verify(&proof)?;
/// ```
pub trait Prover: Send + Sync {
    /// Generate a zero-knowledge proof for an action execution.
    ///
    /// Proves that executing `action` on `before_state` produces `after_state`.
    /// The proof can be verified on-chain or off-chain without revealing the
    /// full game state.
    ///
    /// # Arguments
    ///
    /// * `before_state` - Game state before action execution
    /// * `action` - Action to prove
    /// * `after_state` - Expected game state after action execution
    /// * `delta` - State delta (may be used for optimization hints)
    ///
    /// # Returns
    ///
    /// A `ProofData` containing the serialized proof and backend identifier.
    ///
    /// # Errors
    ///
    /// Returns `ProofError` if:
    /// - Proof generation fails (zkVM error, circuit error, etc.)
    /// - State transition is invalid
    /// - Serialization fails
    fn prove(
        &self,
        before_state: &GameState,
        action: &Action,
        after_state: &GameState,
        delta: &StateDelta,
    ) -> Result<ProofData, ProofError>;

    /// Verify a proof locally (for testing and debugging).
    ///
    /// Checks that a proof is valid according to this prover's verification logic.
    /// This is primarily used for:
    /// - Local testing before submission
    /// - Debugging proof generation issues
    /// - Sanity checks in development
    ///
    /// # Note
    ///
    /// This is a **host-side** verification. For on-chain verification, the proof
    /// must be submitted to a smart contract verifier.
    ///
    /// # Arguments
    ///
    /// * `proof` - The proof to verify
    ///
    /// # Returns
    ///
    /// `Ok(true)` if the proof is valid, `Ok(false)` if invalid.
    ///
    /// # Errors
    ///
    /// Returns `ProofError` if verification itself fails (e.g., deserialization error).
    fn verify(&self, proof: &ProofData) -> Result<bool, ProofError>;
}
