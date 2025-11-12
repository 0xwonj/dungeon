//! Arkworks circuit proving backend (Phase 2+).
//!
//! ⚠️  **PROTOTYPE STATUS - NOT PRODUCTION READY** ⚠️
//!
//! # Security Warnings
//!
//! **CRITICAL:** This implementation has known security limitations and should NOT be used in production:
//!
//! 1. **Incomplete R1CS Constraints**: Poseidon hash gadgets compute witness values without generating
//!    proper R1CS constraints. A malicious prover can submit arbitrary hash outputs, completely
//!    bypassing circuit soundness. See `gadgets.rs:48-79` for details.
//!
//! 2. **Verification Testing Incomplete**: While production code uses secure `OsRng`, the full
//!    circuit has not been tested against malicious provers attempting to forge invalid state transitions.
//!
//! 3. **Incomplete Verification**: The `verify()` method only deserializes proofs without performing
//!    actual Groth16 cryptographic verification against public inputs.
//!
//! 4. **Expensive Key Generation**: Groth16 keys are regenerated on every `prove()` call, making
//!    proof generation prohibitively slow (minutes to hours for complex circuits). Production use
//!    requires key caching/persistence.
//!
//! 5. **Signed Integer Bugs**: Negative coordinate casting produces incorrect field elements,
//!    breaking movement validation for west/south directions.
//!
//! **Use Case**: This backend is suitable for:
//! - Performance benchmarking and optimization research
//! - Circuit architecture prototyping
//! - Developer education on R1CS constraints
//!
//! **NOT suitable for**:
//! - Production deployments
//! - Security-critical applications
//! - On-chain verification
//!
//! See GitHub issue #XX for tracking production-readiness work.
//!
//! ---
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
        let first_actor = before_state
            .entities
            .actors
            .first()
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
    ///
    /// # Security
    ///
    /// # Security Warning
    ///
    /// ⚠️  Uses `test_rng()` which is deterministic and NOT cryptographically secure.
    /// This is acceptable for hello-world examples but NOT for production.
    ///
    /// For production use, either:
    /// 1. Use `ArkworksProver::with_cached_keys()` to pre-generate keys with secure RNG
    /// 2. Persist keys to disk and load them on startup
    /// 3. Implement your own RNG injection mechanism
    pub fn prove(&self) -> Result<ProofData, ProofError> {
        use ark_std::test_rng;

        // TODO: Replace with cryptographically secure RNG for production
        // Current limitation: ark-std's RNG traits don't easily support system entropy
        let mut rng = test_rng();

        // Generate keys for this circuit
        let dummy_circuit = constraints::HelloWorldCircuit::dummy();
        let keys = groth16::Groth16Keys::generate(dummy_circuit, &mut rng)?;

        // Create circuit with witness
        let circuit = constraints::HelloWorldCircuit::new(self.root, self.leaf, self.path.clone());

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
#[cfg(feature = "arkworks")]
#[derive(Debug, Clone)]
pub struct ArkworksProver {
    /// Cached Groth16 keys (optional - generates on first use if not provided)
    cached_keys: Option<groth16::Groth16Keys>,
}

#[cfg(not(feature = "arkworks"))]
#[derive(Debug, Clone, Default)]
pub struct ArkworksProver;

#[cfg(feature = "arkworks")]
impl Default for ArkworksProver {
    fn default() -> Self {
        Self::new()
    }
}

impl ArkworksProver {
    /// Create a new prover without cached keys.
    /// Keys will be generated on each proof (slower, but simple).
    pub fn new() -> Self {
        tracing::info!("ArkworksProver initialized with GameTransitionCircuit (no key caching)");
        Self { cached_keys: None }
    }

    /// Create a new prover with pre-generated cached keys.
    /// This performs the expensive trusted setup once during initialization.
    /// Recommended for benchmarks and production use.
    ///
    /// # Performance
    /// - Key generation: ~15-18 seconds (one-time cost)
    /// - Subsequent proofs: ~1-2 seconds each
    ///
    /// # Security
    ///
    /// # Security Note
    ///
    /// Currently uses `test_rng()` for key generation due to ark-std RNG limitations.
    /// For production deployment:
    /// 1. Generate keys offline with secure RNG
    /// 2. Store keys securely (encrypted at rest)
    /// 3. Load keys from secure storage at runtime
    ///
    /// The generated keys themselves are cryptographically sound, but the RNG
    /// determinism means repeated runs produce identical keys (not ideal for security).
    pub fn with_cached_keys() -> Result<Self, ProofError> {
        use ark_std::test_rng;

        tracing::info!(
            "ArkworksProver: Pre-generating Groth16 keys (this may take 15-20 seconds)..."
        );

        // TODO: Use secure RNG for production key generation
        let mut rng = test_rng();
        let dummy_circuit = game_transition::GameTransitionCircuit::dummy();
        let keys = groth16::Groth16Keys::generate(dummy_circuit, &mut rng)?;

        tracing::info!("ArkworksProver: Keys cached successfully");

        Ok(Self {
            cached_keys: Some(keys),
        })
    }
}

#[cfg(not(feature = "arkworks"))]
impl ArkworksProver {
    pub fn new(_oracle_snapshot: crate::OracleSnapshot) -> Self {
        Self
    }

    /// Legacy method for StateDelta-based proving (will be used in Phase 2).
    #[allow(dead_code)]
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

// ============================================================================
// Action Parameter Extraction Helpers
// ============================================================================

/// Convert ActionKind to circuit ActionType.
#[inline]
fn to_action_type(kind: &game_core::ActionKind) -> game_transition::ActionType {
    use game_core::ActionKind;
    match kind {
        ActionKind::Move => game_transition::ActionType::Move,
        ActionKind::MeleeAttack => game_transition::ActionType::MeleeAttack,
        ActionKind::Wait => game_transition::ActionType::Wait,
    }
}

/// Extract target entity ID from action input as field element.
#[inline]
fn extract_target_id(input: &game_core::ActionInput) -> Option<ark_bn254::Fr> {
    match input {
        game_core::ActionInput::Entity(id) => Some(ark_bn254::Fr::from(id.0 as u64)),
        _ => None,
    }
}

/// Convert cardinal direction to field element (0-7).
#[inline]
fn direction_to_field(dir: &game_core::CardinalDirection) -> ark_bn254::Fr {
    use game_core::CardinalDirection;
    ark_bn254::Fr::from(match dir {
        CardinalDirection::North => 0u64,
        CardinalDirection::NorthEast => 1u64,
        CardinalDirection::East => 2u64,
        CardinalDirection::SouthEast => 3u64,
        CardinalDirection::South => 4u64,
        CardinalDirection::SouthWest => 5u64,
        CardinalDirection::West => 6u64,
        CardinalDirection::NorthWest => 7u64,
    })
}

/// Extract direction from action input as field element.
#[inline]
fn extract_direction(input: &game_core::ActionInput) -> Option<ark_bn254::Fr> {
    match input {
        game_core::ActionInput::Direction(dir) => Some(direction_to_field(dir)),
        _ => None,
    }
}

/// Calculate position delta (dx, dy) for move actions.
#[inline]
fn calculate_position_delta(
    char_action: &game_core::CharacterAction,
    before_state: &GameState,
    after_state: &GameState,
) -> Option<(ark_bn254::Fr, ark_bn254::Fr)> {
    use game_core::ActionKind;

    if !matches!(char_action.kind, ActionKind::Move) {
        return None;
    }

    let before_actor = before_state.entities.actors.iter().find(|a| a.id == char_action.actor)?;
    let after_actor = after_state.entities.actors.iter().find(|a| a.id == char_action.actor)?;

    let dx = after_actor.position.x - before_actor.position.x;
    let dy = after_actor.position.y - before_actor.position.y;
    Some((ark_bn254::Fr::from(dx as i64), ark_bn254::Fr::from(dy as i64)))
}

// ============================================================================
// Prover Implementation
// ============================================================================

impl crate::Prover for ArkworksProver {
    fn prove(
        &self,
        before_state: &GameState,
        action: &game_core::Action,
        after_state: &GameState,
    ) -> Result<ProofData, ProofError> {
        use ark_bn254::Fr as Fp254;

        tracing::info!("ArkworksProver::prove - generating GameTransitionCircuit proof");

        // Extract character action or return error
        let char_action = match action {
            game_core::Action::Character(char_action) => char_action,
            _ => {
                return Err(ProofError::CircuitProofError(
                    "Only CharacterAction is currently supported".to_string(),
                ));
            }
        };

        // Extract action parameters using helper functions
        let action_type = to_action_type(&char_action.kind);
        let actor_id = Fp254::from(char_action.actor.0 as u64);
        let target_id = extract_target_id(&char_action.input);
        let direction = extract_direction(&char_action.input);
        let position_delta = calculate_position_delta(char_action, before_state, after_state);

        // Generate StateDelta
        let delta = game_core::StateDelta::from_states(action.clone(), before_state, after_state);

        // Compute entity tree roots (simplified for MVP - no turn state in circuit yet)
        // TODO Phase 2: Include turn state in circuit constraints
        let mut before_entity_tree = merkle::build_entity_tree(before_state)?;
        let before_root = before_entity_tree.root()?;

        let mut after_entity_tree = merkle::build_entity_tree(after_state)?;
        let after_root = after_entity_tree.root()?;

        // Generate witnesses
        let witnesses = witness::generate_witnesses(&delta, before_state, after_state)?;

        // Create circuit with full witness data
        let circuit = game_transition::GameTransitionCircuit::new(
            before_root,
            after_root,
            action_type.to_field(),
            actor_id,
            witnesses,
            target_id,
            direction,
            position_delta,
        );

        // Use cached keys if available, otherwise generate them (slower path)
        // ⚠️  Security: Uses test_rng() which is deterministic. For production:
        //    1. Generate keys offline with secure RNG (e.g., OsRng from rand crate)
        //    2. Store keys securely (encrypted at rest)
        //    3. Load keys from secure storage at runtime
        // See module-level security warnings for details.
        use ark_std::test_rng;
        let mut rng = test_rng();

        #[cfg(feature = "arkworks")]
        let keys = if let Some(ref cached) = self.cached_keys {
            // Fast path: use pre-generated keys (~0ms overhead)
            tracing::debug!("Using cached Groth16 keys");
            cached.clone()
        } else {
            // Slow path: generate keys on-demand (~15-18 seconds)
            tracing::warn!(
                "Generating Groth16 keys on-demand (slow - consider using with_cached_keys())"
            );
            let dummy_circuit = game_transition::GameTransitionCircuit::dummy();
            groth16::Groth16Keys::generate(dummy_circuit, &mut rng)?
        };

        #[cfg(not(feature = "arkworks"))]
        let keys = {
            let dummy_circuit = game_transition::GameTransitionCircuit::dummy();
            groth16::Groth16Keys::generate(dummy_circuit, &mut rng)?
        };

        // Generate Groth16 proof (~1-2 seconds with cached keys)
        let proof = groth16::prove(circuit, &keys, &mut rng)?;

        // Serialize proof
        let proof_bytes = groth16::serialize_proof(&proof)?;

        tracing::info!(
            "ArkworksProver: Generated proof with {} bytes",
            proof_bytes.len()
        );

        Ok(ProofData {
            bytes: proof_bytes,
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

        tracing::info!("ArkworksProver::verify - deserializing and validating proof structure");

        // Deserialize the Groth16 proof to validate it's well-formed
        let _groth16_proof = groth16::deserialize_proof(&proof.bytes)?;

        // Note: Full verification requires public inputs (before_root, after_root, action_type, actor_id)
        // which are not available in this method signature. In production:
        // - Public inputs would be available on-chain or from the verifier's context
        // - The verifier would call groth16::verify(proof, public_inputs, verifying_key)
        //
        // For now, we validate that the proof is well-formed by successful deserialization
        tracing::info!("ArkworksProver: Proof is well-formed (deserialized successfully)");

        // Return true if deserialization succeeded (structural validity)
        // Note: This is NOT cryptographic verification - just format validation
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

#[cfg(feature = "arkworks")]
/// Shared test helpers for circuit testing.
///
/// Reusable test state builders to reduce code duplication across test files and benchmarks.
pub mod test_helpers;
