//! zkVM proving backend (SP1, RISC0).
//!
//! This module provides a simple interface for proving game actions using zkVMs.
//! Unlike custom circuits, zkVMs automatically generate execution traces and don't
//! require manual witness construction or Merkle tree management.
//!
//! # Design
//!
//! ```text
//! ProverWorker
//!   ↓
//! zkvm::prove(before_state, action, after_state)
//!   ↓
//! zkVM Guest Program (runs in zkVM)
//!   - Execute game logic
//!   - Verify state transition
//!   - Commit public outputs
//!   ↓
//! Proof (contains execution trace)
//! ```
//!
//! # Implementation Status
//!
//! - Phase 2A (current): Stub implementation
//! - Phase 2B: SP1 integration
//! - Phase 2C: RISC0 integration (optional)

use crate::{ProofBackend, ProofData, ProofError};
use game_core::{Action, GameState, StateDelta};

/// zkVM prover interface.
///
/// This trait abstracts over different zkVM backends (SP1, RISC0).
pub trait ZkvmProver: Send + Sync {
    /// Generate a proof that executing `action` on `before_state` produces `after_state`.
    ///
    /// # Arguments
    ///
    /// * `before_state` - Game state before action execution
    /// * `action` - Action to prove
    /// * `after_state` - Expected game state after action execution
    /// * `delta` - State delta (for optimization hints, not strictly required)
    ///
    /// # Returns
    ///
    /// A proof that can be verified on-chain or off-chain.
    fn prove(
        &self,
        before_state: &GameState,
        action: &Action,
        after_state: &GameState,
        delta: &StateDelta,
    ) -> Result<ProofData, ProofError>;

    /// Verify a proof locally (for testing).
    fn verify(&self, proof: &ProofData) -> Result<bool, ProofError>;
}

/// Stub zkVM prover (Phase 2A).
///
/// This is a placeholder that returns dummy proofs until we integrate SP1/RISC0.
pub struct StubZkvmProver;

impl StubZkvmProver {
    pub fn new() -> Self {
        Self
    }
}

impl Default for StubZkvmProver {
    fn default() -> Self {
        Self::new()
    }
}

impl ZkvmProver for StubZkvmProver {
    fn prove(
        &self,
        _before_state: &GameState,
        _action: &Action,
        _after_state: &GameState,
        _delta: &StateDelta,
    ) -> Result<ProofData, ProofError> {
        // Phase 2A: Return a stub proof
        // TODO: Replace with actual SP1 proving once integrated
        Ok(ProofData {
            bytes: vec![0xDE, 0xAD, 0xBE, 0xEF], // Dummy proof
            backend: ProofBackend::Stub,
        })
    }

    fn verify(&self, _proof: &ProofData) -> Result<bool, ProofError> {
        // Phase 2A: Always accept stub proofs
        // TODO: Replace with actual verification
        Ok(true)
    }
}

// Re-export the default prover
pub use StubZkvmProver as DefaultProver;
