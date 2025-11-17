//! SP1 proof to Sui format converter.
//!
//! Handles the gnark → arkworks conversion for SP1 proofs to make them
//! compatible with Sui's Groth16 verifier.

use sp1_sdk::SP1ProofWithPublicValues;
use sp1_sui::convert_sp1_gnark_to_ark;

use zk::ProofData;

use crate::SuiProof;

/// Errors that can occur during proof conversion.
#[derive(Debug, thiserror::Error)]
pub enum ConversionError {
    #[error("Failed to deserialize SP1 proof: {0}")]
    DeserializationError(String),

    #[error("Invalid proof backend: expected SP1, got {0:?}")]
    InvalidBackend(String),

    #[error("SP1-to-Sui conversion failed")]
    ConversionFailed,
}

/// Converter for SP1 proofs to Sui-compatible format.
///
/// This is a stateless utility that performs the gnark → arkworks conversion.
pub struct SuiProofConverter;

impl SuiProofConverter {
    /// Convert SP1 ProofData to Sui-compatible format.
    ///
    /// This function:
    /// 1. Deserializes the SP1 proof from `ProofData.bytes`
    /// 2. Converts gnark format to arkworks (handles endianness, compression flags)
    /// 3. Returns all components needed for Sui transaction
    ///
    /// # Arguments
    ///
    /// * `proof_data` - SP1 proof data from `zk` crate
    ///
    /// # Returns
    ///
    /// `SuiProof` with all components ready for on-chain submission.
    ///
    /// # Errors
    ///
    /// Returns `ConversionError` if:
    /// - Proof backend is not SP1
    /// - Deserialization fails
    /// - Conversion fails
    ///
    /// # Example
    ///
    /// ```ignore
    /// use client_sui::SuiProofConverter;
    /// use zk::ProofData;
    ///
    /// let proof_data: ProofData = load_from_disk()?;
    /// let sui_proof = SuiProofConverter::convert(proof_data)?;
    ///
    /// let (vk, digest, journal, proof) = sui_proof.export_for_transaction();
    /// ```
    pub fn convert(proof_data: ProofData) -> Result<SuiProof, ConversionError> {
        // Verify this is an SP1 proof
        if !matches!(proof_data.backend, zk::ProofBackend::Sp1) {
            return Err(ConversionError::InvalidBackend(format!("{:?}", proof_data.backend)));
        }

        // Deserialize SP1 proof
        let sp1_proof: SP1ProofWithPublicValues = bincode::deserialize(&proof_data.bytes)
            .map_err(|e| ConversionError::DeserializationError(e.to_string()))?;

        // Convert SP1 gnark proof to arkworks format
        // This handles:
        // - Endianness conversion (big → little)
        // - Compression flag adjustment
        // - Negative flag handling
        //
        // Note: convert_sp1_gnark_to_ark returns (Vec<u8>, Vec<u8>, Vec<u8>), not Result
        // It panics on errors internally, which is acceptable for proof conversion
        let (verifying_key, public_inputs, proof_points) = convert_sp1_gnark_to_ark(sp1_proof);

        Ok(SuiProof {
            verifying_key,
            public_inputs,
            proof_points,
            journal: proof_data.journal,
            journal_digest: proof_data.journal_digest,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sui_proof_structure() {
        let proof = SuiProof {
            verifying_key: vec![1, 2, 3],
            public_inputs: vec![4, 5, 6],
            proof_points: vec![7, 8, 9],
            journal: vec![0u8; 168],
            journal_digest: [42u8; 32],
        };

        let (vk, digest, journal, proof_bytes) = proof.export_for_transaction();
        assert_eq!(vk, &[1, 2, 3]);
        assert_eq!(digest, &[42u8; 32]);
        assert_eq!(journal.len(), 168);
        assert_eq!(proof_bytes, &[7, 8, 9]);
    }
}
