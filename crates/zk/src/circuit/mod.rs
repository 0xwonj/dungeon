//! Arkworks circuit proving backend (Phase 2+).
//!
//! This module implements hand-crafted circuits using Arkworks with explicit Merkle witness generation.
//! Uses Poseidon hash for Merkle trees and Groth16 for proof generation.
//! Provides better performance than zkVMs but requires more implementation effort.
//!
//! # Architecture (from docs/state-delta-architecture.md)
//!
//! ```text
//! StateDelta (bitmask)
//!   ↓
//! StateTransition (Merkle witnesses)
//!   ↓
//! Circuit (constraint system)
//!   ↓
//! Proof (compact, fast verification)
//! ```
//!
//! # Modules (to be implemented)
//!
//! - `merkle/`: Sparse Merkle tree implementation
//! - `witness/`: Witness generation from StateDelta
//! - `transition/`: StateTransition structure
//! - `circuits/`: Circuit definitions for each action type
//!
//! # Implementation Status
//!
//! **Not yet implemented** - This is Phase 2+ work that will be done when:
//! 1. zkVM proof generation time becomes a bottleneck
//! 2. On-chain verification costs are too high
//! 3. Team has bandwidth for multi-month circuit development
//!
//! For now, use the zkVM backend (default feature).

#![allow(dead_code)] // Allow dead code since this is future work

use crate::{ProofData, ProofError};
use game_core::{GameState, StateDelta};

#[cfg(feature = "arkworks")]
use ark_bn254::Fr as Fp254;

/// State transition with Merkle witnesses (Phase 2).
///
/// This structure represents a proven state transition with all necessary
/// Merkle proofs for changed entities.
///
/// See: docs/state-delta-architecture.md Section 5.2
#[cfg(not(feature = "arkworks"))]
pub struct StateTransition {
    _placeholder: (),
}

#[cfg(not(feature = "arkworks"))]
impl StateTransition {
    pub fn from_delta(
        _delta: StateDelta,
        _before_state: &GameState,
        _after_state: &GameState,
    ) -> Result<Self, ProofError> {
        Err(ProofError::CircuitProofError(
            "Arkworks circuit not yet implemented - use zkVM backend".to_string(),
        ))
    }

    pub fn prove(&self) -> Result<ProofData, ProofError> {
        Err(ProofError::CircuitProofError(
            "Arkworks circuit not yet implemented - use zkVM backend".to_string(),
        ))
    }
}

/// State transition with Merkle witnesses.
///
/// For the hello world implementation, this contains:
/// - root: The Merkle root hash
/// - leaf: A leaf value to prove
/// - path: The Merkle authentication path
#[cfg(feature = "arkworks")]
#[derive(Clone, Debug)]
pub struct StateTransition {
    pub root: Fp254,
    pub leaf: Fp254,
    pub path: merkle::MerklePath,
}

#[cfg(feature = "arkworks")]
impl StateTransition {
    /// Create a new state transition for hello world proof
    ///
    /// # Arguments
    /// * `root` - The Merkle root hash
    /// * `leaf` - The leaf value to prove
    /// * `path` - The Merkle authentication path
    pub fn new(root: Fp254, leaf: Fp254, path: merkle::MerklePath) -> Self {
        Self { root, leaf, path }
    }

    /// Convert a StateDelta into a StateTransition with Merkle witnesses.
    ///
    /// # Algorithm (from architecture doc)
    ///
    /// 1. Build full Merkle trees from before_state
    /// 2. Build full Merkle trees from after_state
    /// 3. Generate witnesses using delta as guide (only changed entities)
    /// 4. Construct StateTransition with before/after roots and witnesses
    ///
    /// # Complexity
    ///
    /// - Time: O(n log n) where n = entity count
    /// - Space: O(k log n) where k = changed entities
    ///
    /// See: docs/state-delta-architecture.md Section 5.4
    pub fn from_delta(
        delta: StateDelta,
        before_state: &GameState,
        after_state: &GameState,
    ) -> Result<Self, ProofError> {
        // Compute Merkle roots for before and after states
        let before_root = merkle::compute_state_root(before_state)?;
        let _after_root = merkle::compute_state_root(after_state)?;

        // Generate witnesses for all changed entities
        let _witnesses = witness::generate_witnesses(&delta, before_state, after_state)?;

        // For hello world compatibility, use a simple transition
        // TODO: Phase 3 will use full GameTransitionCircuit with all witnesses
        let mut before_tree = merkle::build_entity_tree(before_state)?;

        // Get first actor's leaf for demonstration
        let first_actor = before_state.entities.actors.first()
            .ok_or_else(|| ProofError::StateInconsistency("No actors in state".to_string()))?;
        let leaf_index = first_actor.id.0;
        let leaf_data = merkle::serialize_actor(first_actor);
        let leaf_hash = merkle::hash_many(&leaf_data)?;

        // Generate proof for this leaf
        let path = before_tree.prove(leaf_index)?;

        Ok(Self {
            root: before_root,
            leaf: leaf_hash,
            path,
        })
    }

    /// Generate a Groth16 proof from this transition.
    ///
    /// This creates a circuit, generates proving keys, and produces a proof.
    pub fn prove(&self) -> Result<ProofData, ProofError> {
        use ark_std::test_rng;

        // For hello world, use a test RNG
        // In production, use a cryptographically secure RNG
        let mut rng = test_rng();

        // Generate keys for this circuit
        let dummy_circuit = constraints::HelloWorldCircuit::dummy();
        let keys = groth16::Groth16Keys::generate(dummy_circuit, &mut rng)?;

        // Create circuit with witness
        let circuit = constraints::HelloWorldCircuit::new(
            self.root,
            self.leaf,
            self.path.clone(),
        );

        // Generate proof
        let proof = groth16::prove(circuit, &keys, &mut rng)?;

        // Serialize proof
        let proof_bytes = groth16::serialize_proof(&proof)?;

        Ok(ProofData {
            bytes: proof_bytes,
            backend: crate::ProofBackend::Arkworks,
        })
    }
}

/// Arkworks circuit prover using Groth16 (Phase 2+).
#[derive(Debug, Clone)]
pub struct ArkworksProver {
    #[allow(dead_code)]
    oracle_snapshot: crate::OracleSnapshot,
}

impl ArkworksProver {
    pub fn new(oracle_snapshot: crate::OracleSnapshot) -> Self {
        tracing::warn!(
            "ArkworksProver is a stub implementation - proofs have no cryptographic guarantees"
        );
        Self { oracle_snapshot }
    }

    /// Legacy method for StateDelta-based proving (will be used in Phase 2).
    pub fn prove_delta(
        &self,
        _delta: &StateDelta,
        _before_state: &GameState,
        _after_state: &GameState,
    ) -> Result<ProofData, ProofError> {
        Err(ProofError::CircuitProofError(
            "Arkworks circuit not yet implemented - use zkVM backend".to_string(),
        ))
    }
}

impl crate::Prover for ArkworksProver {
    fn prove(
        &self,
        _before_state: &GameState,
        _action: &game_core::Action,
        _after_state: &GameState,
    ) -> Result<ProofData, ProofError> {
        tracing::warn!("ArkworksProver::prove called - returning dummy proof");

        // Stub implementation returns a placeholder proof
        // Real implementation (Phase 2) will:
        // 1. Generate witness from before_state, action, after_state
        // 2. Compute R1CS constraint satisfaction
        // 3. Generate Groth16 proof
        // 4. Serialize proof for on-chain verification

        Ok(ProofData {
            bytes: vec![0xAA, 0xBB, 0xCC, 0xDD], // Placeholder proof bytes
            backend: crate::ProofBackend::Arkworks,
        })
    }

    fn verify(&self, proof: &ProofData) -> Result<bool, ProofError> {
        if proof.backend != crate::ProofBackend::Arkworks {
            return Err(ProofError::ZkvmError(format!(
                "ArkworksProver can only verify arkworks proofs, got {:?}",
                proof.backend
            )));
        }

        tracing::warn!("ArkworksProver::verify called - returning true (no real verification)");

        // Stub implementation accepts all proofs
        // Real implementation (Phase 2) will:
        // 1. Deserialize Groth16 proof
        // 2. Verify against verifying key
        // 3. Check public inputs match expected state hash

        Ok(true)
    }
}

// Submodules

#[cfg(feature = "arkworks")]
/// Poseidon-based hash functions and state commitments.
///
/// State root computation and commitment schemes using Poseidon hash.
/// See: docs/state-delta-architecture.md Section 5.2
pub mod commitment;

#[cfg(feature = "arkworks")]
/// Sparse Merkle tree implementations.
///
/// Sparse Merkle tree for state commitments using Poseidon hash.
/// See: docs/state-delta-architecture.md Section 5.3
pub mod merkle;

#[cfg(feature = "arkworks")]
/// Witness generation from StateDelta.
///
/// Generate Merkle witnesses from StateDelta.
/// See: docs/state-delta-architecture.md Section 5.4
pub mod witness;

#[cfg(feature = "arkworks")]
/// R1CS constraint generation for state transitions.
///
/// Defines circuits that prove state transition validity.
pub mod constraints;

#[cfg(feature = "arkworks")]
/// Groth16 proof generation and verification.
///
/// Proving key generation, proof creation, and verification.
pub mod groth16;

#[cfg(feature = "arkworks")]
/// Game transition circuit - main ZK circuit for proving game actions.
///
/// Complete circuit implementation with action-specific constraints.
pub mod game_transition;

#[cfg(feature = "arkworks")]
/// R1CS gadgets for state verification.
///
/// Reusable constraint gadgets for Poseidon hashing, Merkle proofs, and validations.
pub mod gadgets;
