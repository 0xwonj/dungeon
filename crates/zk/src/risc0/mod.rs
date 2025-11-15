//! RISC0 zkVM proving backend.
//!
//! Provides STARK proof generation and Groth16 conversion for on-chain verification.
//!
//! # Modules
//!
//! - `prover`: STARK proof generation (Risc0Prover)
//! - `groth16`: Groth16 conversion for on-chain verification (requires Linux x86_64)

mod prover;
pub use prover::Risc0Prover;

pub mod groth16;
