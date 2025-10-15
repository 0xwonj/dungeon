//! zkVM proving backend implementations.
//!
//! Provides zkVM-based provers (RISC0, SP1) that implement the `Prover` trait.
//! zkVMs automatically generate execution traces without manual circuit design.

#[cfg(feature = "risc0")]
mod risc0;
#[cfg(feature = "risc0")]
pub use risc0::Risc0Prover;

// SP1 backend not yet implemented
// #[cfg(feature = "sp1")]
// mod sp1;
// #[cfg(feature = "sp1")]
// pub use sp1::Sp1Prover;
