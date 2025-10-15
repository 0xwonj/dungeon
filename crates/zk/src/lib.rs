//! ZK proof generation utilities.
//!
//! This crate provides a unified interface for different proving backends:
//! - **RISC0** (default): Production-ready zkVM backend
//! - **SP1**: Alternative zkVM backend (not yet implemented)
//! - **Stub**: Dummy prover for fast development iteration
//! - **Arkworks** (future): Custom circuits with Poseidon-based Merkle trees
//!
//! # no_std Support
//!
//! Supports `no_std` by disabling the `std` feature, allowing oracle snapshots
//! and adapters to run inside zkVM guest programs.
//!
//! # Feature Flags
//!
//! Proving backends (mutually exclusive - choose one):
//! - `risc0` (default): RISC0 zkVM backend
//! - `sp1`: SP1 zkVM backend (not implemented)
//! - `stub`: Stub prover returning dummy proofs
//! - No features: Falls back to `stub` automatically
//!
//! Other features:
//! - `std` (default): Enable standard library support
//! - `arkworks` (future): Enable Arkworks circuit proving
//!
//! # Examples
//!
//! ```toml
//! # Default: RISC0 zkVM with std
//! zk = { path = "../zk" }
//!
//! # Guest program (no_std, no prover)
//! zk = { path = "../zk", default-features = false }
//!
//! # Use SP1 zkVM (when implemented)
//! zk = { path = "../zk", default-features = false, features = ["std", "sp1"] }
//!
//! # Use stub prover for fast iteration
//! zk = { path = "../zk", default-features = false, features = ["std", "stub"] }
//!
//! # Arkworks circuit (future)
//! zk = { path = "../zk", features = ["arkworks"] }
//! ```

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

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
#[cfg(feature = "std")]
pub mod prover;

#[cfg(feature = "std")]
pub use prover::{ProofBackend, ProofData, ProofError, Prover};

// zkVM module - zkVM-based proving backend implementations
#[cfg(feature = "std")]
pub mod zkvm;

#[cfg(feature = "std")]
pub use zkvm::*;

// Arkworks circuit module (optional, Phase 2+)
#[cfg(feature = "arkworks")]
pub mod circuit;

#[cfg(feature = "arkworks")]
pub use circuit::*;

// Re-export commonly used types from game-core
pub use game_core::{Action, GameState, StateDelta};
