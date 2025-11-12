//! Circuit-friendly hash functions using Poseidon.
//!
//! Uses ark-crypto-primitives' Poseidon sponge with BN254 field for production security.
//!
//! OPTIMIZATION: Poseidon config is cached globally using OnceLock singleton for ~100-1000x speedup.

#[cfg(feature = "arkworks")]
use ark_bn254::Fr as Fp254;

#[cfg(feature = "arkworks")]
use ark_crypto_primitives::sponge::{
    CryptographicSponge,
    poseidon::{PoseidonConfig, PoseidonSponge, find_poseidon_ark_and_mds},
};

#[cfg(feature = "arkworks")]
use crate::ProofError;

#[cfg(feature = "arkworks")]
use std::sync::OnceLock;

#[cfg(feature = "arkworks")]
/// Global singleton for Poseidon configuration.
///
/// Initialized once on first use and reused for all subsequent hash operations.
/// This eliminates the expensive `find_poseidon_ark_and_mds` computation (which
/// generates round constants and MDS matrices) that was previously done on every
/// hash operation.
///
/// PERFORMANCE: This optimization provides ~100-1000x speedup for hash operations.
static POSEIDON_CONFIG: OnceLock<PoseidonConfig<Fp254>> = OnceLock::new();

#[cfg(feature = "arkworks")]
/// Get Poseidon configuration for BN254 field.
///
/// Uses parameters optimized for security and circuit efficiency.
/// - Full rounds: 8
/// - Partial rounds: 57 (recommended for BN254 field)
/// - Alpha (S-box): 5
/// - Rate: 2
/// - Capacity: 1
///
/// OPTIMIZATION: Configuration is computed once and cached globally.
pub fn get_poseidon_config() -> &'static PoseidonConfig<Fp254> {
    POSEIDON_CONFIG.get_or_init(|| {
        // Standard Poseidon parameters for 128-bit security on BN254
        let full_rounds_u64 = 8u64;
        let partial_rounds_u64 = 57u64;
        let alpha = 5u64;
        let rate_usize = 2usize;
        let capacity_usize = 1usize;

        // Generate round constants and MDS matrix using the standard procedure
        // This is expensive (~10-100ms) but only happens ONCE
        let (ark, mds) = find_poseidon_ark_and_mds::<Fp254>(
            254, // BN254 field size in bits
            rate_usize,
            full_rounds_u64,
            partial_rounds_u64,
            0, // skip matrices (0 = don't skip any)
        );

        PoseidonConfig::new(
            full_rounds_u64 as usize,
            partial_rounds_u64 as usize,
            alpha,
            mds,
            ark,
            rate_usize,
            capacity_usize,
        )
    })
}

#[cfg(feature = "arkworks")]
/// Helper: Squeeze single element from Poseidon sponge.
///
/// Extracts one field element from the sponge state with proper error handling.
#[inline]
fn squeeze_single_element(sponge: &mut PoseidonSponge<Fp254>) -> Result<Fp254, ProofError> {
    sponge
        .squeeze_field_elements::<Fp254>(1)
        .first()
        .copied()
        .ok_or_else(|| ProofError::CircuitProofError("Poseidon squeeze failed".to_string()))
}

#[cfg(feature = "arkworks")]
/// Hash a single field element using Poseidon sponge.
///
/// # Arguments
/// * `input` - Field element to hash
///
/// # Returns
/// * `Ok(Fp254)` - Hash digest on success
/// * `Err(ProofError)` - If Poseidon evaluation fails
///
/// OPTIMIZATION: Uses cached config singleton for ~100-1000x speedup.
pub fn hash_one(input: Fp254) -> Result<Fp254, ProofError> {
    let config = get_poseidon_config();
    let mut sponge = PoseidonSponge::<Fp254>::new(config);

    // Absorb the input (using slice to avoid allocation)
    let inputs = [input];
    sponge.absorb(&inputs.as_slice());

    // Squeeze one field element
    squeeze_single_element(&mut sponge)
}

#[cfg(feature = "arkworks")]
/// Hash two field elements using Poseidon sponge for Merkle tree nodes.
///
/// # Arguments
/// * `left` - Left field element
/// * `right` - Right field element
///
/// # Returns
/// * `Ok(Fp254)` - Hash digest on success
/// * `Err(ProofError)` - If Poseidon evaluation fails
///
/// OPTIMIZATION: Uses cached config singleton and array slice (no heap allocation).
pub fn hash_two(left: Fp254, right: Fp254) -> Result<Fp254, ProofError> {
    let config = get_poseidon_config();
    let mut sponge = PoseidonSponge::<Fp254>::new(config);

    // Absorb both inputs (using array slice to avoid Vec allocation)
    let inputs = [left, right];
    sponge.absorb(&inputs.as_slice());

    // Squeeze one field element
    squeeze_single_element(&mut sponge)
}

#[cfg(feature = "arkworks")]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_one_deterministic() {
        // Test that same input produces same output (determinism)
        let input = Fp254::from(5u64);
        let result1 = hash_one(input).expect("hash_one should succeed");
        let result2 = hash_one(input).expect("hash_one should succeed");
        assert_eq!(result1, result2, "Poseidon hash must be deterministic");
    }

    #[test]
    fn test_hash_one_different_inputs() {
        // Test that different inputs produce different outputs
        let input1 = Fp254::from(5u64);
        let input2 = Fp254::from(42u64);
        let result1 = hash_one(input1).expect("hash_one should succeed");
        let result2 = hash_one(input2).expect("hash_one should succeed");
        assert_ne!(
            result1, result2,
            "Different inputs must produce different hashes"
        );
    }

    #[test]
    fn test_hash_two_deterministic() {
        // Test that same inputs produce same output (determinism)
        let left = Fp254::from(3u64);
        let right = Fp254::from(4u64);
        let result1 = hash_two(left, right).expect("hash_two should succeed");
        let result2 = hash_two(left, right).expect("hash_two should succeed");
        assert_eq!(result1, result2, "Poseidon hash must be deterministic");
    }

    #[test]
    fn test_hash_two_different_inputs() {
        // Test that different inputs produce different outputs
        let result1 =
            hash_two(Fp254::from(3u64), Fp254::from(4u64)).expect("hash_two should succeed");
        let result2 =
            hash_two(Fp254::from(5u64), Fp254::from(6u64)).expect("hash_two should succeed");
        assert_ne!(
            result1, result2,
            "Different inputs must produce different hashes"
        );
    }

    #[test]
    fn test_hash_two_order_matters() {
        // Test that order matters: hash(a, b) != hash(b, a)
        let left = Fp254::from(3u64);
        let right = Fp254::from(4u64);
        let result1 = hash_two(left, right).expect("hash_two should succeed");
        let result2 = hash_two(right, left).expect("hash_two should succeed");
        assert_ne!(result1, result2, "Hash should not be commutative");
    }

    #[test]
    fn test_hash_one_and_two_different() {
        // Test that hash_one and hash_two produce different results for the same value
        let value = Fp254::from(5u64);
        let hash1 = hash_one(value).expect("hash_one should succeed");
        let hash2 = hash_two(value, value).expect("hash_two should succeed");
        // These should be different since they use different arity
        // (one input vs two inputs, even if the inputs are the same value)
        assert_ne!(
            hash1, hash2,
            "hash_one and hash_two should produce different outputs"
        );
    }
}
