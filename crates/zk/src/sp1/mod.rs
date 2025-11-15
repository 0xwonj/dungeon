//! SP1 zkVM backend module.
//!
//! Provides SP1-specific proving functionality with platform-independent
//! Groth16 support.
//!
//! # Proof Types
//!
//! The proof type is determined at **compile time** via feature flags:
//!
//! - **`sp1`** (default): Compressed STARK proof (~4-5MB, off-chain verification)
//! - **`sp1-groth16`**: Groth16 SNARK proof (~260 bytes, on-chain verification, ~270k gas)
//!
//! Both use the same `prove()` method - the proof type is selected by cargo features.
//!
//! # Platform Support
//!
//! Unlike RISC0, SP1's Groth16 works on **all platforms**:
//! - macOS (Intel and Apple Silicon)
//! - Linux (x86_64 and ARM)
//! - Windows
//!
//! # Example Usage
//!
//! ```bash
//! # Off-chain verification (development/testing)
//! cargo build --no-default-features --features sp1
//!
//! # On-chain verification (blockchain deployment)
//! cargo build --no-default-features --features sp1-groth16
//! ```
//!
//! ```ignore
//! use zk::sp1::Sp1Prover;
//!
//! let prover = Sp1Prover::new(oracle_snapshot);
//!
//! // Same API - proof type selected by feature flag
//! let proof = prover.prove(&start, &actions, &end)?;
//!
//! // With sp1: ~4-5MB Compressed proof
//! // With sp1-groth16: ~260 bytes Groth16 proof
//! ```

mod prover;
pub use prover::Sp1Prover;
