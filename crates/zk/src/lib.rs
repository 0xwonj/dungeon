//! ZK proof generation utilities.
//!
//! This crate supports multiple proving modes:
//! - **RISC0** (default): RISC0 zkVM backend for production
//! - **SP1**: Alternative zkVM backend
//! - **zkvm**: Stub prover for development/testing without real backend
//! - **arkworks** (future): Hand-crafted circuits with Merkle witnesses using Poseidon hash
//!
//! # Feature Flags
//!
//! - (default): RISC0 zkVM backend
//! - `risc0`: Use RISC0 zkVM backend (default)
//! - `sp1`: Use SP1 zkVM backend
//! - `zkvm`: Stub prover for development/testing
//! - `arkworks`: Enable Arkworks circuit proving with Poseidon-based Merkle trees
//!
//! # Examples
//!
//! ```toml
//! # Default: RISC0 zkVM
//! zk = { path = "../zk" }
//!
//! # Use SP1 zkVM instead
//! zk = { path = "../zk", default-features = false, features = ["sp1"] }
//!
//! # Use stub prover for testing
//! zk = { path = "../zk", default-features = false, features = ["zkvm"] }
//!
//! # Use Arkworks circuit with Poseidon
//! zk = { path = "../zk", features = ["arkworks"] }
//!
//! # Hybrid: both zkVM and Arkworks circuit
//! zk = { path = "../zk", features = ["arkworks"] }
//! ```

// Oracle snapshot for serializable game content
pub mod oracle;
pub use oracle::{
    ConfigSnapshot, ItemsSnapshot, MapSnapshot, NpcsSnapshot, OracleSnapshot,
    OracleSnapshotBuilder, TablesSnapshot,
};

// zkVM module (optional)
#[cfg(feature = "zkvm")]
pub mod zkvm;

#[cfg(feature = "zkvm")]
pub use zkvm::*;

// Arkworks circuit module (optional, Phase 2+)
#[cfg(feature = "arkworks")]
pub mod circuit;

#[cfg(feature = "arkworks")]
pub use circuit::*;

// Re-export commonly used types
pub use game_core::{Action, GameState, StateDelta};

/// ZK proof data container.
///
/// The internal representation depends on the enabled backend:
/// - zkVM: Contains SP1/RISC0 proof bytes
/// - Arkworks: Contains Groth16 proof bytes
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProofData {
    /// Proof bytes (format depends on backend)
    pub bytes: Vec<u8>,

    /// Backend identifier
    pub backend: ProofBackend,
}

/// Identifies which proving backend was used
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ProofBackend {
    /// No proof generated (game-only mode)
    None,

    /// Stub prover (used when no real backend is available)
    #[cfg(feature = "zkvm")]
    Stub,

    #[cfg(feature = "sp1")]
    Sp1,

    #[cfg(feature = "risc0")]
    Risc0,

    #[cfg(feature = "arkworks")]
    Arkworks,
}

/// Errors that can occur during proof generation
#[derive(Debug, thiserror::Error)]
pub enum ProofError {
    /// zkVM proof generation failed
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
    #[error("State inconsistency: {0}")]
    StateInconsistency(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),
}
