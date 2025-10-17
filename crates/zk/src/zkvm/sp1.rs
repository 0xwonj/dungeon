//! SP1 zkVM prover implementation (not yet implemented).
//!
//! This module will provide an SP1 zkVM backend for proof generation.
//! SP1 is an alternative zkVM to RISC0 with similar capabilities.
//!
//! # Status
//!
//! This implementation is planned for future development.
//!
//! # Architecture (Planned)
//!
//! Similar to RISC0 implementation:
//! - Host-side prover builds execution environment
//! - Guest program executes game logic inside SP1 zkVM
//! - Proof receipt contains journal with public outputs
//!
//! # Usage (When Implemented)
//!
//! ```toml
//! [dependencies]
//! zk = { path = "../zk", default-features = false, features = ["std", "sp1"] }
//! ```

use crate::prover::{ProofBackend, ProofData, ProofError};
use crate::OracleSnapshot;
use game_core::{Action, GameState};

/// SP1 zkVM prover implementation.
///
/// **Not yet implemented** - this is a placeholder for future development.
pub struct Sp1Prover {
    /// Cached oracle snapshot (immutable game content)
    #[allow(dead_code)]
    oracle_snapshot: OracleSnapshot,
}

impl Sp1Prover {
    /// Creates a new SP1 prover with oracle snapshot.
    ///
    /// # Panics
    ///
    /// Panics immediately - SP1 backend is not yet implemented.
    pub fn new(_oracle_snapshot: OracleSnapshot) -> Self {
        unimplemented!("SP1 prover backend is not yet implemented")
    }
}

impl crate::Prover for Sp1Prover {
    fn prove(
        &self,
        _before_state: &GameState,
        _action: &Action,
        _after_state: &GameState,
    ) -> Result<ProofData, ProofError> {
        Err(ProofError::ZkvmError(
            "SP1 prover backend is not yet implemented".to_string(),
        ))
    }

    fn verify(&self, _proof: &ProofData) -> Result<bool, ProofError> {
        Err(ProofError::ZkvmError(
            "SP1 prover backend is not yet implemented".to_string(),
        ))
    }
}
