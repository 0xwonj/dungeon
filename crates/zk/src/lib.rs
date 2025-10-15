//! ZK proof generation utilities (host-side only).
//!
//! Provides a unified interface for different proving backends:
//! - **RISC0** (default): Production zkVM backend
//! - **SP1**: Alternative zkVM backend (not implemented)
//! - **Stub**: Dummy prover for testing
//! - **Arkworks** (future): Custom circuit proving
//!
//! This crate is for host-side proof generation only. Guest programs should
//! depend on `game-core` directly.
//!
//! # Feature Flags
//!
//! Backends are mutually exclusive - enable exactly one:
//! - `risc0` (default): RISC0 zkVM backend
//! - `sp1`: SP1 zkVM backend
//! - `stub`: Stub prover for testing
//! - `arkworks`: Arkworks circuit proving (future)

// Feature conflict checks
#[cfg(any(
    all(feature = "risc0", feature = "sp1"),
    all(feature = "risc0", feature = "stub"),
    all(feature = "risc0", feature = "arkworks"),
    all(feature = "sp1", feature = "stub"),
    all(feature = "sp1", feature = "arkworks"),
    all(feature = "stub", feature = "arkworks")
))]
compile_error!("Enable exactly one backend: risc0, sp1, stub, or arkworks");

#[cfg(not(any(
    feature = "risc0",
    feature = "sp1",
    feature = "stub",
    feature = "arkworks"
)))]
compile_error!("Enable exactly one backend: risc0, sp1, stub, or arkworks");

// Include generated methods from build.rs (RISC0 ELF and ImageID)
// The build script generates this file with the guest program binary
#[cfg(feature = "risc0")]
mod generated {
    // Try to include generated methods, fall back to placeholders if not available
    include!(concat!(env!("OUT_DIR"), "/methods.rs"));
}

#[cfg(feature = "risc0")]
pub use generated::{GAME_VERIFIER_ELF, GAME_VERIFIER_ID};

// Oracle snapshot for serializable game content
pub mod oracle;
pub use oracle::{
    ConfigSnapshot, ItemsSnapshot, MapSnapshot, NpcsSnapshot, OracleSnapshot, TablesSnapshot,
};

// Prover module - universal interface and types for all proving backends
pub mod prover;
pub use prover::{ProofBackend, ProofData, ProofError, Prover};

#[cfg(feature = "stub")]
pub use prover::StubProver;

// zkVM module - zkVM-based proving backend implementations
pub mod zkvm;
pub use zkvm::*;

// Arkworks circuit module (optional, Phase 2+)
#[cfg(feature = "arkworks")]
pub mod circuit;

#[cfg(feature = "arkworks")]
pub use circuit::*;

// Re-export commonly used types from game-core
pub use game_core::{Action, GameState, StateDelta};

/// The ZK prover backend configured for this build.
///
/// This type alias resolves to the appropriate prover based on enabled features:
/// - `risc0` (default) → Risc0Prover
/// - `sp1` (not implemented) → Sp1Prover
/// - `stub` → StubProver (testing only)
#[cfg(feature = "risc0")]
pub type ZkProver = Risc0Prover;

#[cfg(feature = "sp1")]
pub type ZkProver = Sp1Prover;

#[cfg(feature = "stub")]
pub type ZkProver = crate::prover::StubProver;
