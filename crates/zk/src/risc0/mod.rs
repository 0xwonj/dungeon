//! RISC0 zkVM proving backend.
//!
//! Provides STARK proof generation for zero-knowledge proofs.
//!
//! # Modules
//!
//! - `prover`: STARK proof generation (Risc0Prover)

mod prover;
pub use prover::Risc0Prover;
