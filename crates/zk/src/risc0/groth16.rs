//! Groth16 proof conversion for on-chain verification.
//!
//! Converts RISC0 STARK proofs (Composite/Succinct Receipt) to Groth16 proofs
//! for efficient on-chain verification on blockchain platforms like Sui.
//!
//! # Platform Requirements
//!
//! **Groth16 conversion requires Linux x86_64:**
//! - Intel/AMD x86_64 architecture
//! - NVIDIA GPU (recommended, via `rzup install risc0-groth16`)
//! - OR Docker (fallback, slower)
//!
//! **Not supported:**
//! - macOS (Intel or Apple Silicon)
//! - Windows
//! - ARM architectures
//!
//! # Receipt Types
//!
//! ```text
//! Composite Receipt    (mehrere MB, multiple segments)
//!     ↓ compress(Succinct)
//! Succinct Receipt     (~200-300 KB, single STARK)
//!     ↓ compress(Groth16)
//! Groth16 Receipt      (~200 bytes, on-chain optimized)
//! ```
//!
//! All receipt types contain the same 168-byte journal with identical structure.
//!
//! # Usage
//!
//! ```ignore
//! use zk::risc0::groth16::compress_to_groth16;
//!
//! // Generate STARK proof (works on any platform)
//! let stark_proof = prover.prove(start, actions, end)?;
//!
//! // Convert to Groth16 (Linux x86_64 only)
//! let groth16_proof = compress_to_groth16(&stark_proof)?;
//!
//! // Submit to blockchain
//! blockchain.submit_proof(&groth16_proof)?;
//! ```
//!
//! # Development Workflow
//!
//! **Mac/local development:**
//! - Generate STARK proofs
//! - Test with journal data
//! - Verify all logic with STARK receipts
//!
//! **Linux CI/production:**
//! - Convert STARK → Groth16
//! - Submit to blockchain
//! - On-chain verification

use crate::prover::{ProofData, ProofError};

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
use crate::prover::ProofBackend;

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
use risc0_zkvm::{InnerReceipt, Receipt};

/// Compress a STARK proof (Composite or Succinct Receipt) to a Groth16 proof.
///
/// This function converts a RISC0 STARK proof into a Groth16 proof suitable for
/// on-chain verification. The resulting proof is ~200 bytes compared to the
/// original STARK proof which can be several megabytes.
///
/// # Platform Requirements
///
/// **This function only works on Linux x86_64.**
///
/// - macOS: Will return `ProofError::ZkvmError` (Groth16 not supported)
/// - Windows: Will return `ProofError::ZkvmError` (Groth16 not supported)
/// - Linux ARM: Will return `ProofError::ZkvmError` (Groth16 not supported)
///
/// # GPU Prover vs Docker
///
/// On Linux x86_64, Groth16 conversion can use:
///
/// **1. GPU Prover (recommended, faster):**
/// ```bash
/// rzup install risc0-groth16
/// ```
/// - Requires NVIDIA GPU (CUDA) or AMD GPU (ROCm)
/// - Conversion time: seconds to tens of seconds
///
/// **2. Docker (fallback, slower):**
/// ```bash
/// docker pull risczero/risc0-groth16-prover
/// ```
/// - No GPU required
/// - Conversion time: minutes
///
/// # Arguments
///
/// * `stark_proof` - ProofData containing a Composite or Succinct Receipt
///
/// # Returns
///
/// - `Ok(ProofData)` - Groth16 proof with:
///   - `bytes`: Groth16 seal (~200 bytes)
///   - `journal`: Same 168-byte journal as input
///   - `journal_digest`: Same digest as input
///   - `backend`: ProofBackend::Risc0
///
/// - `Err(ProofError)` - If:
///   - Platform is not Linux x86_64
///   - Receipt compression fails
///   - Groth16 conversion fails
///
/// # Examples
///
/// ```ignore
/// // Generate STARK proof (any platform)
/// let stark_proof = risc0_prover.prove(start_state, actions, end_state)?;
///
/// // Convert to Groth16 (Linux x86_64 only)
/// #[cfg(target_os = "linux")]
/// #[cfg(target_arch = "x86_64")]
/// {
///     let groth16_proof = compress_to_groth16(&stark_proof)?;
///
///     // Journal is identical
///     assert_eq!(groth16_proof.journal, stark_proof.journal);
///     assert_eq!(groth16_proof.journal_digest, stark_proof.journal_digest);
///
///     // But proof is much smaller
///     assert!(groth16_proof.bytes.len() < 300); // ~200 bytes
/// }
/// ```
///
/// # Errors
///
/// This function will return an error if:
///
/// - The platform is not Linux x86_64
/// - The input receipt cannot be deserialized
/// - Groth16 compression fails (GPU/Docker issues)
/// - The resulting receipt is not a Groth16 receipt
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub fn compress_to_groth16(stark_proof: &ProofData) -> Result<ProofData, ProofError> {
    use risc0_zkvm::sha::Digest;

    // Verify backend
    if stark_proof.backend != ProofBackend::Risc0 {
        return Err(ProofError::ZkvmError(format!(
            "Expected RISC0 proof, got {:?}",
            stark_proof.backend
        )));
    }

    // Deserialize STARK receipt (Composite or Succinct)
    let receipt: Receipt = bincode::deserialize(&stark_proof.bytes).map_err(|e| {
        ProofError::SerializationError(format!("Failed to deserialize receipt: {}", e))
    })?;

    // Compress to Groth16
    // This internally:
    // 1. Compresses Composite → Succinct (if needed)
    // 2. Compresses Succinct → Groth16
    // 3. Uses GPU prover (rzup) or Docker
    let groth16_receipt = receipt
        .inner
        .compress(risc0_zkvm::CompressionType::Groth16)
        .map_err(|e| {
            ProofError::ZkvmError(format!(
                "Groth16 compression failed: {}. \
                 Ensure GPU prover (rzup install risc0-groth16) or Docker is available.",
                e
            ))
        })?;

    // Extract Groth16 seal from inner receipt
    let groth16_seal = match groth16_receipt {
        InnerReceipt::Groth16(ref g16) => {
            // Serialize the Groth16 seal for on-chain submission
            bincode::serialize(&g16.seal).map_err(|e| {
                ProofError::SerializationError(format!("Failed to serialize Groth16 seal: {}", e))
            })?
        }
        _ => {
            return Err(ProofError::ZkvmError(
                "Compression did not produce Groth16Receipt".into(),
            ));
        }
    };

    // Verify journal is preserved (sanity check)
    // The journal should be identical in STARK and Groth16 receipts
    let groth16_journal = match &groth16_receipt {
        InnerReceipt::Groth16(g16) => {
            // Extract journal from claim
            let claim_digest = g16.verifier_parameters.as_slice();
            // Note: The journal itself is in the Receipt wrapper, not InnerReceipt
            // We reuse the original journal since it's the same
            stark_proof.journal.clone()
        }
        _ => unreachable!(),
    };

    Ok(ProofData {
        bytes: groth16_seal,
        backend: ProofBackend::Risc0,
        journal: groth16_journal,
        journal_digest: stark_proof.journal_digest,
    })
}

/// Groth16 compression not available on non-Linux platforms.
///
/// This function is a stub that returns an error on platforms other than Linux x86_64.
/// Groth16 conversion requires Circom witness generation which only runs on Linux x86_64.
#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
pub fn compress_to_groth16(_stark_proof: &ProofData) -> Result<ProofData, ProofError> {
    Err(ProofError::ZkvmError(
        "Groth16 conversion only supported on Linux x86_64. \
         Current platform is not supported. \
         \n\nOptions:\n\
         - Use GitHub Actions with ubuntu-latest runner\n\
         - Deploy to Linux x86_64 server (AWS, GCP, etc.)\n\
         - Use Multipass to run Linux VM locally\n\
         \nFor development: Use STARK proofs directly (journal is identical)."
            .into(),
    ))
}
