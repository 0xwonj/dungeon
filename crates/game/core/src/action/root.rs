//! Action root computation for ZK proofs.
//!
//! This module provides functionality to compute a cryptographic commitment
//! to a sequence of actions, used for ZK proof generation and verification.

#[cfg(feature = "serde")]
use super::Action;

/// Computes actions root for ZK proofs
///
/// # Arguments
///
/// * `actions` - Slice of actions to compute root for
///
/// # Returns
///
/// A 32-byte SHA-256 hash of the serialized actions
///
/// # Production Migration
///
/// When integrating with Walrus storage:
/// 1. Store action sequence on Walrus, get blob_id
/// 2. Use blob_id directly as actions_root
/// 3. On-chain verification checks blob_id matches proof
/// 4. Verifiers can fetch actions from Walrus using blob_id
///
/// # Serialization
///
/// Requires the `serde` feature. Works in both std and no_std (zkvm) environments.
#[cfg(feature = "serde")]
pub fn compute_actions_root(actions: &[Action]) -> [u8; 32] {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();

    // Hash each action in sequence order
    // Important: Order matters for reproducibility
    for action in actions {
        // bincode serialization is deterministic and consistent
        if let Ok(action_bytes) = bincode::serialize(action) {
            hasher.update(&action_bytes);
        }
    }

    hasher.finalize().into()
}
