//! Arkworks circuit proving backend (Phase 2+).
//!
//! ⚠️  **PROTOTYPE STATUS - NOT PRODUCTION READY** ⚠️
//!
//! # Security Warnings
//!
//! **CRITICAL:** This implementation has known security limitations and should NOT be used in production:
//!
//! 1. **Verification Testing Incomplete**: The circuit has not been tested against malicious provers
//!    attempting to forge invalid state transitions. While R1CS constraints are properly generated
//!    (verified by tests in `gadgets.rs:348-408`) and cryptographic verification is now implemented,
//!    comprehensive adversarial testing is required before production use.
//!
//! 2. **Key Management**: Production deployments need key persistence and proper key distribution
//!    to verifiers. Keys are now generated using secure RNG (OsRng) in production builds.
//!
//! 3. **Expensive Key Generation**: Groth16 keys are regenerated on every `prove()` call unless
//!    `with_cached_keys()` is used, making proof generation prohibitively slow (minutes for complex
//!    circuits). Production use requires key caching/persistence.
//!
//! 4. **Signed Integer Bugs**: Negative coordinate casting may produce incorrect field elements,
//!    breaking movement validation for west/south directions. Needs testing with negative coordinates.
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
use game_core::GameState;

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
    ///
    /// ⚠️  **CRITICAL LIMITATION**: Groth16 keys are circuit-specific. Cached keys
    /// are generated using `dummy()` circuit which has exactly 1 entity witness.
    /// These keys will ONLY work for proofs with exactly 1 entity changing.
    /// Proofs with 0, 2, or more entity changes will have different circuit structures
    /// and verification will fail!
    ///
    /// **Recommendation**: Don't use this method. Let `new()` generate keys per-proof
    /// with matching circuit structure. Yes, it's slow (~15-18s per proof), but it works.
    ///
    /// # Why Key Caching is Broken
    ///
    /// Groth16's proving/verifying keys are tied to the specific constraint system.
    /// Our circuit structure changes based on how many entities are modified:
    /// - Move action: usually 1 entity (the actor)
    /// - Attack action: 2 entities (attacker + target)
    /// - Different witness counts = different constraint systems = different keys needed
    ///
    /// To fix this properly, we'd need to either:
    /// 1. Pad all circuits to a fixed maximum witness count (wasteful)
    /// 2. Generate and cache multiple key sets for different witness counts
    /// 3. Use a universal SNARK that allows circuit flexibility
    ///
    /// # Performance
    /// - Key generation: ~15-18 seconds (one-time cost)
    /// - Subsequent proofs: ~1-2 seconds each (but only for 1-witness circuits!)
    ///
    /// # Security
    ///
    /// Uses cryptographically secure RNG (OsRng) for key generation in production builds.
    /// Test builds use deterministic test_rng() for reproducibility.
    ///
    /// For production deployment, keys should be:
    /// 1. Generated offline with secure RNG (done automatically with OsRng)
    /// 2. Stored securely (encrypted at rest)
    /// 3. Loaded from secure storage at runtime
    ///
    /// The generated keys themselves are cryptographically sound.
    #[deprecated(note = "Key caching only works for 1-entity proofs. Use new() instead.")]
    pub fn with_cached_keys() -> Result<Self, ProofError> {
        tracing::warn!(
            "ArkworksProver::with_cached_keys() is deprecated - keys only work for 1-entity circuits. \
             Consider using new() instead for correct per-proof key generation."
        );

        tracing::info!(
            "ArkworksProver: Pre-generating Groth16 keys with dummy() circuit (this may take 15-20 seconds)..."
        );

        // Use secure RNG in production, deterministic in tests
        #[cfg(not(test))]
        use rand::rngs::OsRng;
        #[cfg(not(test))]
        let mut rng = OsRng;

        #[cfg(test)]
        use ark_std::test_rng;
        #[cfg(test)]
        let mut rng = test_rng();

        let dummy_circuit = game_transition::GameTransitionCircuit::dummy();
        let keys = groth16::Groth16Keys::generate(dummy_circuit, &mut rng)?;

        tracing::info!("ArkworksProver: Keys cached successfully (only usable for 1-entity proofs)");

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

    let before_actor = before_state
        .entities
        .actors
        .iter()
        .find(|a| a.id == char_action.actor)?;
    let after_actor = after_state
        .entities
        .actors
        .iter()
        .find(|a| a.id == char_action.actor)?;

    let dx = after_actor.position.x - before_actor.position.x;
    let dy = after_actor.position.y - before_actor.position.y;
    Some((
        ark_bn254::Fr::from(dx as i64),
        ark_bn254::Fr::from(dy as i64),
    ))
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
        // Note: We need to clone witnesses because Groth16 requires generating keys
        // with the EXACT same circuit structure (same number of witnesses)
        let circuit = game_transition::GameTransitionCircuit::new(
            before_root,
            after_root,
            action_type.to_field(),
            actor_id,
            witnesses.clone(),
            target_id,
            direction,
            position_delta,
        );

        // Use secure RNG for key generation (OsRng from system entropy)
        // In tests, use deterministic test_rng() for reproducibility
        #[cfg(not(test))]
        use rand::rngs::OsRng;
        #[cfg(not(test))]
        let mut rng = OsRng;

        #[cfg(test)]
        use ark_std::test_rng;
        #[cfg(test)]
        let mut rng = test_rng();

        #[cfg(feature = "arkworks")]
        let keys = if let Some(ref cached) = self.cached_keys {
            // Fast path: use pre-generated keys (~0ms overhead)
            // ⚠️  LIMITATION: Cached keys only work if the circuit structure matches dummy()
            //     which means exactly 1 entity witness. If your proof has different number
            //     of witnesses, this will fail verification!
            tracing::debug!("Using cached Groth16 keys (only works for circuits matching dummy structure)");
            cached.clone()
        } else {
            // Correct path: generate keys with same circuit structure as proof
            // This is slow (~15-18 seconds) but produces correct keys
            tracing::warn!(
                "Generating Groth16 keys for circuit with {} entity witnesses (~15-18s)",
                witnesses.entities.len()
            );

            // CRITICAL: Generate keys using identical circuit structure
            let key_gen_circuit = game_transition::GameTransitionCircuit::new(
                before_root,
                after_root,
                action_type.to_field(),
                actor_id,
                witnesses,
                target_id,
                direction,
                position_delta,
            );
            groth16::Groth16Keys::generate(key_gen_circuit, &mut rng)?
        };

        #[cfg(not(feature = "arkworks"))]
        let keys = {
            let key_gen_circuit = game_transition::GameTransitionCircuit::new(
                before_root,
                after_root,
                action_type.to_field(),
                actor_id,
                witnesses,
                target_id,
                direction,
                position_delta,
            );
            groth16::Groth16Keys::generate(key_gen_circuit, &mut rng)?
        };

        // Generate Groth16 proof (~1-2 seconds)
        let proof = groth16::prove(circuit, &keys, &mut rng)?;

        // Serialize proof
        let proof_bytes = groth16::serialize_proof(&proof)?;

        // Serialize public inputs for verification
        // Public inputs: [before_root, after_root, action_type, actor_id]
        use ark_serialize::CanonicalSerialize;
        let public_inputs_fields = vec![before_root, after_root, action_type.to_field(), actor_id];
        let mut public_inputs_bytes = Vec::new();
        for field in &public_inputs_fields {
            let mut field_bytes = Vec::new();
            field
                .serialize_compressed(&mut field_bytes)
                .map_err(|e| ProofError::SerializationError(e.to_string()))?;
            public_inputs_bytes.push(field_bytes);
        }

        // Serialize verifying key for verification
        // In production, the verifying key would be a public constant, not stored with each proof
        // But for our prototype, we include it for self-contained verification
        let vk_bytes = keys.serialize_verifying_key()?;

        tracing::info!(
            "ArkworksProver: Generated proof with {} bytes, {} public inputs, vk {} bytes",
            proof_bytes.len(),
            public_inputs_bytes.len(),
            vk_bytes.len()
        );

        Ok(ProofData {
            bytes: proof_bytes,
            backend: crate::ProofBackend::Arkworks,
            public_inputs: Some(public_inputs_bytes),
            verifying_key: Some(vk_bytes),
        })
    }

    fn verify(&self, proof: &ProofData) -> Result<bool, ProofError> {
        if proof.backend != crate::ProofBackend::Arkworks {
            return Err(ProofError::ZkvmError(format!(
                "ArkworksProver can only verify arkworks proofs, got {:?}",
                proof.backend
            )));
        }

        tracing::info!("ArkworksProver::verify - performing cryptographic verification");

        // Deserialize the Groth16 proof
        let groth16_proof = groth16::deserialize_proof(&proof.bytes)?;

        // Extract public inputs (required for Groth16 verification)
        let public_inputs_bytes = proof.public_inputs.as_ref().ok_or_else(|| {
            ProofError::CircuitProofError(
                "Arkworks proof missing public inputs - cannot verify".to_string(),
            )
        })?;

        // Deserialize public inputs from bytes
        use ark_bn254::Fr as Fp254;
        use ark_serialize::CanonicalDeserialize;
        let mut public_inputs = Vec::new();
        for field_bytes in public_inputs_bytes {
            let field = Fp254::deserialize_compressed(field_bytes.as_slice())
                .map_err(|e| ProofError::SerializationError(e.to_string()))?;
            public_inputs.push(field);
        }

        tracing::info!(
            "ArkworksProver: Deserialized {} public inputs for verification",
            public_inputs.len()
        );

        // Get verifying key from proof data
        let vk_bytes = proof.verifying_key.as_ref().ok_or_else(|| {
            ProofError::CircuitProofError(
                "Arkworks proof missing verifying key - cannot verify".to_string(),
            )
        })?;

        // Deserialize verifying key
        let vk = groth16::Groth16Keys::deserialize_verifying_key(vk_bytes)?;

        tracing::debug!("Using verifying key from proof data");

        // Perform actual Groth16 cryptographic verification
        let is_valid = groth16::verify(&groth16_proof, &public_inputs, &vk)?;

        if is_valid {
            tracing::info!("ArkworksProver: Proof verified successfully ✓");
        } else {
            tracing::warn!("ArkworksProver: Proof verification FAILED ✗");
        }

        Ok(is_valid)
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
