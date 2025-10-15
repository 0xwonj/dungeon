//! zkVM proving backend implementations.
//!
//! This module provides zkVM-based proving backends that implement the `Prover` trait.
//! zkVMs automatically generate execution traces without requiring manual witness
//! construction or circuit design.
//!
//! # Available Backends
//!
//! - **Risc0Prover** (feature: `risc0`): Production-ready RISC0 zkVM backend
//! - **Sp1Prover** (feature: `sp1`): SP1 zkVM backend (not yet implemented)
//!
//! # Backend Selection
//!
//! Use the `ZkProver` type alias to get the configured backend at compile time.
//! The backend is selected via Cargo features.

use crate::prover::{ProofBackend, ProofData, ProofError};
use game_core::{Action, GameState, StateDelta};

/// Stub prover for testing and development.
///
/// Returns dummy proofs without performing actual proof generation.
/// All proofs verify successfully. Use this for:
/// - Fast iteration during development
/// - Testing without zkVM infrastructure
/// - CI environments where proof generation is not needed
///
/// # Warning
///
/// Do not use in production - provides no cryptographic guarantees.
#[derive(Debug, Clone, Copy, Default)]
pub struct StubProver;

impl StubProver {
    /// Creates a new stub prover instance.
    pub fn new() -> Self {
        Self
    }
}

impl crate::Prover for StubProver {
    fn prove(
        &self,
        _before_state: &GameState,
        _action: &Action,
        _after_state: &GameState,
        _delta: &StateDelta,
    ) -> Result<ProofData, ProofError> {
        // Return recognizable dummy bytes for debugging
        Ok(ProofData {
            bytes: vec![0xDE, 0xAD, 0xBE, 0xEF],
            backend: ProofBackend::Stub,
        })
    }

    fn verify(&self, proof: &ProofData) -> Result<bool, ProofError> {
        // Only verify stub proofs
        if proof.backend != ProofBackend::Stub {
            return Err(ProofError::ZkvmError(format!(
                "StubProver can only verify stub proofs, got {:?}",
                proof.backend
            )));
        }
        Ok(true)
    }
}

// Backend implementations (conditionally compiled)
#[cfg(feature = "risc0")]
mod risc0;
#[cfg(feature = "risc0")]
pub use risc0::Risc0Prover;

// SP1 backend not yet implemented
// #[cfg(feature = "sp1")]
// mod sp1;
// #[cfg(feature = "sp1")]
// pub use sp1::Sp1Prover;

// ============================================================================
// ZkProver type alias - compile-time backend selection
// ============================================================================
//
// This type alias resolves to the configured proving backend based on enabled
// features. Use this in your code instead of concrete prover types to allow
// flexible backend selection at compile time.
//
// Selection priority:
// 1. risc0 feature → Risc0Prover
// 2. sp1 feature → Sp1Prover (when implemented)
// 3. No backend features → StubProver (safe fallback)
//
// # Example
//
// ```rust
// use zk::{Prover, zkvm::ZkProver};
//
// let prover = ZkProver::new(oracle_snapshot);
// let proof = prover.prove(&before, &action, &after, &delta)?;
// ```

#[cfg(feature = "risc0")]
/// The ZK prover backend configured for this build.
///
/// Currently using: **RISC0 zkVM** (feature: `risc0`)
pub type ZkProver = Risc0Prover;

#[cfg(all(not(feature = "risc0"), feature = "sp1"))]
/// The ZK prover backend configured for this build.
///
/// Currently using: **SP1 zkVM** (feature: `sp1`)
pub type ZkProver = Sp1Prover;

#[cfg(all(not(feature = "risc0"), not(feature = "sp1")))]
/// The ZK prover backend configured for this build.
///
/// Currently using: **Stub prover** (no backend features enabled)
pub type ZkProver = StubProver;
