//! Hashing utilities for game state and proofs.
//!
//! Provides deterministic hashing for logging and debugging purposes.

use std::hash::{Hash, Hasher};

use game_core::GameState;
use zk::ProofData;

/// Compute a deterministic hash of GameState using bincode serialization.
///
/// Returns the first 8 bytes of the hash as a hex string for compact logging.
pub fn hash_game_state(state: &GameState) -> String {
    let bytes = bincode::serialize(state).expect("GameState serialization should not fail");
    let hash = hash_bytes(&bytes);
    format!("{:016x}", hash)
}

/// Compute a hash of proof data bytes.
///
/// Returns the first 8 bytes of the hash as a hex string for compact logging.
pub fn hash_proof_data(proof: &ProofData) -> String {
    let hash = hash_bytes(&proof.bytes);
    format!("{:016x}", hash)
}

/// Compute a 64-bit hash from bytes using a fast, deterministic hasher.
fn hash_bytes(bytes: &[u8]) -> u64 {
    let mut hasher = std::hash::DefaultHasher::new();
    bytes.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_consistency() {
        let state = GameState::default();
        let hash1 = hash_game_state(&state);
        let hash2 = hash_game_state(&state);
        assert_eq!(hash1, hash2, "Same state should produce same hash");
    }

    #[test]
    fn test_hash_format() {
        let state = GameState::default();
        let hash = hash_game_state(&state);
        assert_eq!(hash.len(), 16, "Hash should be 16 hex chars (8 bytes)");
        assert!(
            hash.chars().all(|c| c.is_ascii_hexdigit()),
            "Hash should only contain hex digits"
        );
    }
}
