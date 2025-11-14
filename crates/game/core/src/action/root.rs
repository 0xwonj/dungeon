//! Action root computation for ZK proofs.
//!
//! This module provides functionality to compute a cryptographic commitment
//! to a sequence of actions, used for ZK proof generation and verification.

use super::Action;

/// Computes actions root for ZK proofs - simulates Walrus blob_id.
///
/// This function creates a cryptographic commitment to a sequence of actions.
/// In production, this would be replaced by the actual Walrus blob_id where
/// the action sequence is stored on decentralized storage.
///
/// # Arguments
///
/// * `actions` - Slice of actions to compute root for
///
/// # Returns
///
/// A 32-byte SHA-256 hash of the serialized actions
///
/// # Design
///
/// - Uses bincode for deterministic serialization
/// - SHA-256 provides cryptographic commitment to the entire sequence
/// - Hardware-accelerated in RISC0 zkVM (when using RISC0's sha2 fork)
/// - Order matters: Hash is computed sequentially over actions
///
/// # Usage
///
/// This function is designed to work in both host and guest (zkVM) environments:
///
/// ```ignore
/// use game_core::action::compute_actions_root;
///
/// // Host side (runtime)
/// let actions = vec![action1, action2, action3];
/// let actions_root = compute_actions_root(&actions);
///
/// // Guest side (zkVM)
/// let actions: Vec<Action> = env::read();
/// let actions_root = compute_actions_root(&actions);
/// env::commit(&actions_root);
/// ```
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
