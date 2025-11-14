//! Poseidon hash functions for BN254 field elements.
//!
//! Provides native (non-circuit) Poseidon hashing for:
//! - Merkle tree construction
//! - Leaf hash computation
//! - State commitment generation
//!
//! # Performance
//!
//! Uses globally cached Poseidon config (OnceLock) for ~100-1000x speedup.
//! Config is initialized once on first use and reused for all subsequent hashing.
//!
//! # Security Parameters
//!
//! - Field: BN254 (254-bit prime)
//! - Full rounds: 8
//! - Partial rounds: 57
//! - Security level: 128 bits
//!
//! # Consistency with Circuit
//!
//! Hash functions must match their circuit gadget counterparts in `gadgets.rs`:
//! - `hash_one()` ↔ `poseidon_hash_one_gadget()`
//! - `hash_two()` ↔ `poseidon_hash_two_gadget()`
//! - `hash_many()` ↔ `poseidon_hash_many_gadget()`
//!
//! Validated by consistency tests in `tests/poseidon_consistency.rs`.

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
/// Cached Poseidon config (initialized once, ~100-1000x faster than recomputing).
static POSEIDON_CONFIG: OnceLock<PoseidonConfig<Fp254>> = OnceLock::new();

#[cfg(feature = "arkworks")]
/// Get cached Poseidon config (8/57 rounds, 128-bit security).
pub fn get_poseidon_config() -> &'static PoseidonConfig<Fp254> {
    POSEIDON_CONFIG.get_or_init(|| {
        let (ark, mds) = find_poseidon_ark_and_mds::<Fp254>(254, 2, 8, 57, 0);
        PoseidonConfig::new(8, 57, 5, mds, ark, 2, 1)
    })
}

#[cfg(feature = "arkworks")]
#[inline]
fn squeeze_single_element(sponge: &mut PoseidonSponge<Fp254>) -> Result<Fp254, ProofError> {
    sponge
        .squeeze_field_elements::<Fp254>(1)
        .first()
        .copied()
        .ok_or_else(|| ProofError::CircuitProofError("Poseidon squeeze failed".to_string()))
}

#[cfg(feature = "arkworks")]
pub fn hash_one(input: Fp254) -> Result<Fp254, ProofError> {
    let mut sponge = PoseidonSponge::<Fp254>::new(get_poseidon_config());
    let inputs = [input];
    sponge.absorb(&inputs.as_slice());
    squeeze_single_element(&mut sponge)
}

#[cfg(feature = "arkworks")]
/// CRITICAL: Absorbs left/right separately to match gadget (absorbing together produces different hashes).
pub fn hash_two(left: Fp254, right: Fp254) -> Result<Fp254, ProofError> {
    let mut sponge = PoseidonSponge::<Fp254>::new(get_poseidon_config());
    let inputs = [left];
    sponge.absorb(&inputs.as_slice());
    let inputs = [right];
    sponge.absorb(&inputs.as_slice());
    squeeze_single_element(&mut sponge)
}

#[cfg(feature = "arkworks")]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_one_deterministic() {
        let input = Fp254::from(5u64);
        let result1 = hash_one(input).expect("hash_one should succeed");
        let result2 = hash_one(input).expect("hash_one should succeed");
        assert_eq!(result1, result2);
    }

    #[test]
    fn test_hash_one_different_inputs() {
        let result1 = hash_one(Fp254::from(5u64)).expect("hash_one should succeed");
        let result2 = hash_one(Fp254::from(42u64)).expect("hash_one should succeed");
        assert_ne!(result1, result2);
    }

    #[test]
    fn test_hash_two_deterministic() {
        let (left, right) = (Fp254::from(3u64), Fp254::from(4u64));
        let result1 = hash_two(left, right).expect("hash_two should succeed");
        let result2 = hash_two(left, right).expect("hash_two should succeed");
        assert_eq!(result1, result2);
    }

    #[test]
    fn test_hash_two_different_inputs() {
        let result1 =
            hash_two(Fp254::from(3u64), Fp254::from(4u64)).expect("hash_two should succeed");
        let result2 =
            hash_two(Fp254::from(5u64), Fp254::from(6u64)).expect("hash_two should succeed");
        assert_ne!(result1, result2);
    }

    #[test]
    fn test_hash_two_order_matters() {
        let (left, right) = (Fp254::from(3u64), Fp254::from(4u64));
        let result1 = hash_two(left, right).expect("hash_two should succeed");
        let result2 = hash_two(right, left).expect("hash_two should succeed");
        assert_ne!(result1, result2);
    }

    #[test]
    fn test_hash_one_and_two_different() {
        let value = Fp254::from(5u64);
        let hash1 = hash_one(value).expect("hash_one should succeed");
        let hash2 = hash_two(value, value).expect("hash_two should succeed");
        assert_ne!(hash1, hash2);
    }
}
