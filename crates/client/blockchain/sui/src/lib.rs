//! Sui blockchain integration for Dungeon game.
//!
//! This crate handles proof submission to Sui blockchain, including:
//! - SP1 proof format conversion (gnark → arkworks)
//! - Verifying key deployment
//! - Proof verification transaction construction
//! - Game session management on-chain
//!
//! # Architecture
//!
//! The conversion from SP1 to Sui format happens at the client layer,
//! keeping the `zk` crate pure and blockchain-agnostic:
//!
//! ```text
//! zk crate (ProofData) → client-sui → Sui blockchain
//!                          ↓
//!                     gnark→arkworks
//!                     conversion
//! ```
//!
//! # Usage
//!
//! ```ignore
//! use client_sui::SuiProofSubmitter;
//! use zk::ProofData;
//!
//! // Load proof from persistence
//! let proof_data = load_proof_from_disk()?;
//!
//! // Convert and submit to Sui
//! let submitter = SuiProofSubmitter::new(sui_client).await?;
//! let tx_digest = submitter.submit_proof(vk_object_id, proof_data).await?;
//! ```

pub mod converter;
pub mod submitter;

pub use converter::SuiProofConverter;
pub use submitter::SuiProofSubmitter;

/// Sui-compatible proof components ready for on-chain submission.
#[derive(Debug, Clone)]
pub struct SuiProof {
    /// Arkworks-serialized verifying key
    pub verifying_key: Vec<u8>,

    /// Public inputs (journal digest as 32-byte SHA-256 hash)
    pub public_inputs: Vec<u8>,

    /// Arkworks-serialized proof points
    pub proof_points: Vec<u8>,

    /// 168-byte journal data (public values from zkVM)
    pub journal: Vec<u8>,

    /// SHA-256 digest of journal (the actual Groth16 public input)
    pub journal_digest: [u8; 32],
}

impl SuiProof {
    /// Export all components as a tuple for Sui transaction.
    ///
    /// Returns `(vk_bytes, journal_digest, journal_data, proof_bytes)`.
    /// This matches the signature of `verify_game_proof()` in the Move contract.
    pub fn export_for_transaction(&self) -> (&[u8], &[u8; 32], &[u8], &[u8]) {
        (
            &self.verifying_key,
            &self.journal_digest,
            &self.journal,
            &self.proof_points,
        )
    }
}
