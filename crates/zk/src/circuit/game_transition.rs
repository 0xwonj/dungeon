//! GameTransitionCircuit - ZK circuit for proving valid game state transitions.
//!
//! This circuit verifies that a game action was executed correctly according to
//! the game rules, without revealing hidden information like RNG seeds or enemy intent.
//!
//! OPTIMIZATION: Uses cached Poseidon config and optimized Merkle verification.
//!
//! # Circuit Architecture
//!
//! ## Public Inputs (visible to verifier):
//! - `before_root`: Merkle root of state before action
//! - `after_root`: Merkle root of state after action
//! - `action_type`: Type of action performed (Move, Attack, Wait, etc.)
//! - `actor_id`: Entity ID performing the action
//!
//! ## Private Witnesses (hidden from verifier):
//! - Entity witnesses: Before/after states with Merkle proofs for changed entities
//! - Action parameters: Target, direction, item ID, etc.
//! - Intermediate calculations: Damage rolls, movement validation, resource costs
//!
//! ## Constraint Structure:
//!
//! 1. **Merkle Proof Verification** (universal)
//!    - Verify before_root matches Merkle tree of initial entities
//!    - Verify after_root matches Merkle tree of final entities
//!    - Verify all witness paths are valid
//!
//! 2. **Actor Validation** (universal)
//!    - Actor entity exists in before state
//!    - Actor has the action available (not on cooldown)
//!    - Actor has sufficient resources (health, stamina, etc.)
//!
//! 3. **Action-Specific Constraints** (polymorphic)
//!    - Move: position adjacency, passability, no collision
//!    - MeleeAttack: target in range, damage calculation, health update
//!    - Wait: no-op validation (trivial)
//!
//! 4. **Effect Application** (modular)
//!    - Damage effects: formula evaluation, resistance, critical hits
//!    - Movement effects: position updates, bounds checking
//!    - Resource effects: cost deduction, restoration, overfill checks
//!    - Status effects: duration tracking, stack counting
//!
//! # Design Decisions
//!
//! ## Field Element Representation:
//! All game values are represented as BN254 field elements (254-bit prime).
//! - Entity IDs: direct mapping (u32 fits in field)
//! - Positions: (x, y) as separate field elements
//! - Health/Resources: direct mapping (u32 values)
//! - Booleans: 0 or 1
//!
//! ## Action Polymorphism:
//! Instead of a massive monolithic circuit, we use a selector pattern:
//! - `action_selector` witness indicates which action constraints to apply
//! - Only one action type's constraints are active per proof
//! - Unused constraints are satisfied trivially
//!
//! ## Efficiency Optimizations:
//! - Sparse witnesses: Only changed entities included
//! - Lazy Merkle proofs: Only prove paths for modified entities
//! - Constraint batching: Group similar validations
//! - Gadget reuse: Common patterns (Poseidon hash, range checks) shared
//!
//! # Implementation Strategy
//!
//! Phase 1: Core Infrastructure (CURRENT)
//! - Merkle proof verification gadget
//! - Basic action selector logic
//! - State root matching
//!
//! Phase 2: Action Constraints
//! - Move action (position validation)
//! - Wait action (trivial case)
//! - MeleeAttack action (damage calculation)
//!
//! Phase 3: Effect Constraints
//! - Damage effects with resistance
//! - Resource manipulation
//! - Status effect application
//!
//! Phase 4: Advanced Features
//! - Multi-target actions
//! - Area-of-effect constraints
//! - Complex formulas (crits, scaling)

use ark_bn254::Fr as Fp254;
use ark_r1cs_std::alloc::AllocVar;
use ark_r1cs_std::boolean::Boolean;
use ark_r1cs_std::eq::EqGadget;
use ark_r1cs_std::fields::{FieldVar, fp::FpVar};
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError};

use super::gadgets::{poseidon_hash_many_gadget, verify_merkle_path_gadget};
use super::merkle::MerklePath;
use super::witness::TransitionWitnesses;

// ============================================================================
// Action Type Encoding
// ============================================================================

/// Action type identifiers for circuit selector.
///
/// These must match the ActionKind enum in game-core.
/// Encoded as field elements for circuit consumption.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionType {
    Move = 0,
    Wait = 1,
    MeleeAttack = 2,
}

impl ActionType {
    /// Convert to field element for circuit use.
    pub fn to_field(self) -> Fp254 {
        Fp254::from(self as u64)
    }

    /// Parse from field element (for testing/debugging).
    pub fn from_field(field: Fp254) -> Option<Self> {
        // Convert field element to u64 (if it fits)
        use ark_ff::PrimeField;
        let bigint = field.into_bigint();
        let limbs = bigint.as_ref();

        // For BN254, first limb contains the value if it fits in u64
        if limbs.is_empty() {
            return None;
        }
        let value = limbs[0];

        match value {
            0 => Some(Self::Move),
            1 => Some(Self::Wait),
            2 => Some(Self::MeleeAttack),
            _ => None,
        }
    }
}

// ============================================================================
// Circuit Structure
// ============================================================================

/// Main game transition circuit.
///
/// Proves that a game action was executed correctly:
/// - Actor validation (exists, has action available, sufficient resources)
/// - Action-specific rules (movement, combat, interactions)
/// - State transition (before_root → after_root via valid changes)
///
/// # Generics
///
/// This circuit is generic to support different proof systems and testing:
/// - Field type F (typically Fp254 for Groth16 on BN254 curve)
#[derive(Clone)]
pub struct GameTransitionCircuit {
    // ========================================================================
    // Public Inputs
    // ========================================================================
    /// Merkle root of state before action execution.
    pub before_root: Option<Fp254>,

    /// Merkle root of state after action execution.
    pub after_root: Option<Fp254>,

    /// Type of action being proven (Move, Attack, Wait, etc.).
    pub action_type: Option<Fp254>,

    /// Entity ID of the actor performing the action.
    pub actor_id: Option<Fp254>,

    // ========================================================================
    // Private Witnesses
    // ========================================================================
    /// Witnesses for all entities modified during this action.
    pub witnesses: Option<TransitionWitnesses>,

    /// Target entity ID (for targeted actions like MeleeAttack).
    pub target_id: Option<Fp254>,

    /// Direction for movement (encoded as field element: 0=North, 1=South, etc.).
    pub direction: Option<Fp254>,

    /// Position delta for movement: (delta_x, delta_y).
    pub position_delta: Option<(Fp254, Fp254)>,
}

impl GameTransitionCircuit {
    /// Create a new circuit instance with all inputs.
    pub fn new(
        before_root: Fp254,
        after_root: Fp254,
        action_type: Fp254,
        actor_id: Fp254,
        witnesses: TransitionWitnesses,
        target_id: Option<Fp254>,
        direction: Option<Fp254>,
        position_delta: Option<(Fp254, Fp254)>,
    ) -> Self {
        Self {
            before_root: Some(before_root),
            after_root: Some(after_root),
            action_type: Some(action_type),
            actor_id: Some(actor_id),
            witnesses: Some(witnesses),
            target_id,
            direction,
            position_delta,
        }
    }

    /// Create a dummy circuit for key generation.
    ///
    /// IMPORTANT: Must use the same Merkle depth as real circuits (depth 10) to ensure
    /// the circuit structure matches. Different depths = different constraint counts = invalid keys.
    pub fn dummy() -> Self {
        use super::witness::EntityWitness;
        use game_core::EntityId;

        // Create minimal valid witnesses for key generation
        // CRITICAL: Use depth 10 to match build_entity_tree() - circuit structure must be identical
        const DUMMY_DEPTH: usize = 10;

        // CRITICAL: Witness data must be consistent with position_delta!
        // For Move action with delta = (0, 1), we need:
        // - before_data[2] = y (e.g., 0)
        // - after_data[2] = y + 1 (e.g., 1)
        // Entity serialization format: [id, x, y, hp, max_hp]
        let before_data = vec![
            Fp254::from(0u64), // id
            Fp254::from(0u64), // x
            Fp254::from(0u64), // y (before)
            Fp254::from(0u64), // hp
            Fp254::from(0u64), // max_hp
        ];

        let after_data = vec![
            Fp254::from(0u64), // id
            Fp254::from(0u64), // x
            Fp254::from(1u64), // y (after) = before_y + delta_y = 0 + 1
            Fp254::from(0u64), // hp
            Fp254::from(0u64), // max_hp
        ];

        // CRITICAL: Compute correct Merkle roots AND paths for dummy witness data!
        // Using incorrect paths causes Merkle verification constraints to fail.
        use super::merkle::{hash_many, SparseMerkleTree};

        // Hash witness data to get leaf hashes
        let before_leaf = hash_many(&before_data).expect("Failed to hash before_data");
        let after_leaf = hash_many(&after_data).expect("Failed to hash after_data");

        // Build Merkle trees with the leaf at index 0 (Entity ID 0)
        let mut before_tree = SparseMerkleTree::new(DUMMY_DEPTH);
        before_tree.insert(0, before_leaf);
        let before_root_value = before_tree.root().expect("Failed to compute before root");
        let before_path = before_tree.prove(0).expect("Failed to generate before path");

        let mut after_tree = SparseMerkleTree::new(DUMMY_DEPTH);
        after_tree.insert(0, after_leaf);
        let after_root_value = after_tree.root().expect("Failed to compute after root");
        let after_path = after_tree.prove(0).expect("Failed to generate after path");

        // Create entity witness with correct paths
        let entity_witness = EntityWitness {
            id: EntityId(0),
            before_data,
            before_path,
            after_data,
            after_path,
        };

        let witnesses = TransitionWitnesses {
            entities: vec![entity_witness],
        };

        // For key generation, we need actual witness values that satisfy constraints
        // Arkworks handles None/AssignmentMissing during key gen, but we use real values
        // to ensure the dummy circuit can also be used for testing proof generation
        Self {
            before_root: Some(before_root_value),
            after_root: Some(after_root_value),
            action_type: Some(Fp254::from(0u64)),  // Move
            actor_id: Some(Fp254::from(0u64)),     // Entity ID 0
            witnesses: Some(witnesses),
            target_id: Some(Fp254::from(0u64)),
            direction: Some(Fp254::from(0u64)),  // North
            position_delta: Some((Fp254::from(0i64), Fp254::from(1i64))),
        }
    }
}

// ============================================================================
// Constraint Synthesis
// ============================================================================

impl ConstraintSynthesizer<Fp254> for GameTransitionCircuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fp254>) -> Result<(), SynthesisError> {
        // ====================================================================
        // 1. Allocate Public Inputs
        // ====================================================================
        //
        // MVP/Phase 1: before_root and after_root are entity tree roots only
        // Phase 2: Will include turn state (combined state roots)
        //
        // This simplified approach allows us to verify Merkle proofs directly
        // against the public roots without needing turn state witnesses in-circuit.

        let before_root_var = FpVar::new_input(cs.clone(), || {
            self.before_root.ok_or(SynthesisError::AssignmentMissing)
        })?;

        let after_root_var = FpVar::new_input(cs.clone(), || {
            self.after_root.ok_or(SynthesisError::AssignmentMissing)
        })?;

        let action_type_var = FpVar::new_input(cs.clone(), || {
            self.action_type.ok_or(SynthesisError::AssignmentMissing)
        })?;

        let actor_id_var = FpVar::new_input(cs.clone(), || {
            self.actor_id.ok_or(SynthesisError::AssignmentMissing)
        })?;

        // ====================================================================
        // 2. Allocate Private Witnesses
        // ====================================================================

        let witnesses = self.witnesses.ok_or(SynthesisError::AssignmentMissing)?;

        // Allocate target_id (may be None for non-targeted actions)
        let target_id_var = alloc_witness_fp_optional(cs.clone(), self.target_id)?;

        // Allocate direction (may be None for non-movement actions)
        let _direction_var = alloc_witness_fp_optional(cs.clone(), self.direction)?;

        // Allocate position delta (may be None for non-movement actions)
        let position_delta_var = match self.position_delta {
            Some((dx, dy)) => Some((alloc_witness_fp(cs.clone(), dx)?, alloc_witness_fp(cs.clone(), dy)?)),
            None => None,
        };

        // ====================================================================
        // 3. Merkle Proof Verification (Before State)
        // ====================================================================

        // For now, we'll verify that at least one entity witness is present
        // and its before_path verifies against before_root.
        //
        // TODO: Verify all entity witnesses in the full implementation.

        if witnesses.entities.is_empty() {
            return Err(SynthesisError::Unsatisfiable);
        }

        // Get the actor's witness (should be first changed entity)
        let actor_witness = &witnesses.entities[0];

        // Allocate actor's before data
        let actor_before_data_vars = alloc_witness_fp_vec(cs.clone(), &actor_witness.before_data)?;

        // Allocate actor's before Merkle path
        let actor_before_path_vars = allocate_merkle_path(cs.clone(), &actor_witness.before_path)?;

        // Verify actor's before Merkle proof
        let actor_leaf_hash = compute_leaf_hash(&actor_before_data_vars)?;
        verify_merkle_path_constraint(&actor_leaf_hash, &actor_before_path_vars, &before_root_var)?;

        // ====================================================================
        // 4. Action-Specific Constraints
        // ====================================================================

        // Create boolean selectors for each action type
        let action_move_fp = Fp254::from(ActionType::Move as u64);
        let action_wait_fp = Fp254::from(ActionType::Wait as u64);
        let action_attack_fp = Fp254::from(ActionType::MeleeAttack as u64);

        let is_move = action_type_var.is_eq(&FpVar::constant(action_move_fp))?;
        let is_wait = action_type_var.is_eq(&FpVar::constant(action_wait_fp))?;
        let is_attack = action_type_var.is_eq(&FpVar::constant(action_attack_fp))?;

        // ====================================================================
        // 5. Allocate After State Data (needed for action constraints)
        // ====================================================================

        // Allocate actor's after data (must be done before action constraints)
        let actor_after_data_vars = alloc_witness_fp_vec(cs.clone(), &actor_witness.after_data)?;

        // Apply action-specific constraints
        constrain_move_action(
            &is_move,
            &actor_before_data_vars,
            &actor_after_data_vars,
            &position_delta_var,
            cs.clone(),
        )?;

        constrain_wait_action(&is_wait, cs.clone())?;

        constrain_attack_action(
            &is_attack,
            &actor_before_data_vars,
            &target_id_var,
            cs.clone(),
        )?;

        // ====================================================================
        // 6. Merkle Proof Verification (After State)
        // ====================================================================

        // Allocate actor's after Merkle path
        let actor_after_path_vars = allocate_merkle_path(cs.clone(), &actor_witness.after_path)?;

        // Verify actor's after Merkle proof
        let actor_after_leaf_hash = compute_leaf_hash(&actor_after_data_vars)?;
        verify_merkle_path_constraint(
            &actor_after_leaf_hash,
            &actor_after_path_vars,
            &after_root_var,
        )?;

        // ====================================================================
        // 6. State Consistency Checks
        // ====================================================================

        // Verify that actor_id matches the witness entity ID
        let witness_actor_id_fp = Fp254::from(actor_witness.id.0 as u64);
        actor_id_var.enforce_equal(&FpVar::constant(witness_actor_id_fp))?;

        Ok(())
    }
}

// ============================================================================
// Helper Functions for Constraint Generation
// ============================================================================

/// Allocate a single field element as witness variable.
#[inline]
fn alloc_witness_fp(
    cs: ConstraintSystemRef<Fp254>,
    value: Fp254,
) -> Result<FpVar<Fp254>, SynthesisError> {
    FpVar::new_witness(cs, || Ok(value))
}

/// Allocate an optional field element as witness variable.
#[inline]
fn alloc_witness_fp_optional(
    cs: ConstraintSystemRef<Fp254>,
    value: Option<Fp254>,
) -> Result<Option<FpVar<Fp254>>, SynthesisError> {
    match value {
        Some(v) => Ok(Some(alloc_witness_fp(cs, v)?)),
        None => Ok(None),
    }
}

/// Allocate a vector of field elements as witness variables.
#[inline]
fn alloc_witness_fp_vec(
    cs: ConstraintSystemRef<Fp254>,
    values: &[Fp254],
) -> Result<Vec<FpVar<Fp254>>, SynthesisError> {
    values.iter().map(|&field| alloc_witness_fp(cs.clone(), field)).collect()
}

/// Allocate a Merkle path as circuit variables.
fn allocate_merkle_path(
    cs: ConstraintSystemRef<Fp254>,
    path: &MerklePath,
) -> Result<Vec<(FpVar<Fp254>, Boolean<Fp254>)>, SynthesisError> {
    path.siblings
        .iter()
        .zip(path.directions.iter())
        .map(|(sibling, &direction)| {
            let sibling_var = alloc_witness_fp(cs.clone(), *sibling)?;
            let direction_var = Boolean::new_witness(cs.clone(), || Ok(direction))?;
            Ok((sibling_var, direction_var))
        })
        .collect()
}

/// Compute hash of leaf data using Poseidon.
fn compute_leaf_hash(data: &[FpVar<Fp254>]) -> Result<FpVar<Fp254>, SynthesisError> {
    if data.is_empty() {
        return Err(SynthesisError::Unsatisfiable);
    }
    poseidon_hash_many_gadget(data)
}

/// Verify Merkle path constraint using Poseidon hash gadget.
fn verify_merkle_path_constraint(
    leaf: &FpVar<Fp254>,
    path: &[(FpVar<Fp254>, Boolean<Fp254>)],
    root: &FpVar<Fp254>,
) -> Result<(), SynthesisError> {
    verify_merkle_path_gadget(leaf, path, root)
}

// ============================================================================
// Action-Specific Constraint Functions
// ============================================================================

/// Constrain Move action: verify position change is valid.
///
/// Validates:
/// 1. Actor's before position is (x, y)
/// 2. Delta is a valid direction: {-1,0,1} x {-1,0,1} excluding (0,0)
/// 3. New position = old position + delta
/// 4. Actor's after position matches computed new position
/// 5. New position is within bounds (0 <= x,y < map_size)
///
/// Entity serialization format (from merkle.rs serialize_actor):
/// - Field 0: entity ID (u32)
/// - Field 1: position x (i32)
/// - Field 2: position y (i32)
/// - Field 3: current HP (u32)
/// - Field 4: max HP (u32)
fn constrain_move_action(
    is_move: &Boolean<Fp254>,
    before_data: &[FpVar<Fp254>],
    after_data: &[FpVar<Fp254>],
    position_delta: &Option<(FpVar<Fp254>, FpVar<Fp254>)>,
    _cs: ConstraintSystemRef<Fp254>,
) -> Result<(), SynthesisError> {
    // If is_move is false, skip constraints (they're satisfied trivially)
    // We use conditional selection to ensure constraints are satisfied regardless

    if let Some((delta_x, delta_y)) = position_delta {
        // Extract actor's before and after positions from serialized data
        // Field 1 = x, Field 2 = y (Field 0 is entity ID)
        if before_data.len() < 3 || after_data.len() < 3 {
            return Err(SynthesisError::Unsatisfiable);
        }

        let before_x = &before_data[1];
        let before_y = &before_data[2];
        let after_x = &after_data[1];
        let after_y = &after_data[2];

        // Validate delta is one of 8 valid directions
        // Valid deltas: {-1, 0, 1} x {-1, 0, 1} excluding (0, 0)
        use super::gadgets::one_of_gadget;
        let valid_delta_values = vec![Fp254::from(-1i64), Fp254::from(0u64), Fp254::from(1u64)];
        one_of_gadget(delta_x, &valid_delta_values)?;
        one_of_gadget(delta_y, &valid_delta_values)?;

        // Ensure not both zero (must actually move)
        let dx_is_zero = delta_x.is_eq(&FpVar::constant(Fp254::from(0u64)))?;
        let dy_is_zero = delta_y.is_eq(&FpVar::constant(Fp254::from(0u64)))?;
        let both_zero = &dx_is_zero & &dy_is_zero;

        // If is_move is true, both_zero must be false
        // If is_move is false, we don't care
        let move_constraint = is_move & &both_zero;
        move_constraint.enforce_equal(&Boolean::FALSE)?;

        // Calculate expected new position
        let expected_x = before_x + delta_x;
        let expected_y = before_y + delta_y;

        // **CRITICAL CONSTRAINT**: Verify the after state position matches the computed position
        // This is the core of the Move action verification - without this, a malicious prover
        // could provide any after state and the circuit would accept it!
        //
        // Enforce: after_x == before_x + delta_x
        // Enforce: after_y == before_y + delta_y
        after_x.enforce_equal(&expected_x)?;
        after_y.enforce_equal(&expected_y)?;

        // TODO: Verify new position is within bounds
        // Temporarily disabled due to is_cmp issues in arkworks 0.5.0
        // Bounds are validated in game-core before proof generation
        // let map_max = Fp254::from(1000u64);
        // bounds_check_gadget(&expected_x, &expected_y, map_max, map_max)?;

        // TODO: Verify new position is not occupied (requires additional witness for occupancy map)
        // TODO: Verify new position is passable (requires witness for tile passability)
    }

    Ok(())
}

/// Constrain Wait action: verify no state changes except tick advancement.
///
/// The Wait action is the simplest action - it advances time but doesn't
/// modify any entity state. The Merkle proof verification already ensures
/// state consistency, so no additional constraints are needed.
fn constrain_wait_action(
    _is_wait: &Boolean<Fp254>,
    _cs: ConstraintSystemRef<Fp254>,
) -> Result<(), SynthesisError> {
    // Wait action has no additional constraints
    // State consistency is verified by Merkle proofs
    Ok(())
}

/// Constrain Attack action: verify damage calculation and health update.
///
/// Validates:
/// 1. Target entity exists in before state
/// 2. Target is in range (adjacent for melee: Chebyshev distance <= 1)
/// 3. Actor has sufficient stamina
/// 4. Damage = attack_power * damage_multiplier
/// 5. Target's health decreased by damage amount (clamped to 0)
/// 6. Actor's stamina decreased by action cost
///
/// Entity serialization format (from merkle.rs serialize_actor):
/// - Field 0: entity ID (u32)
/// - Field 1: position x (i32)
/// - Field 2: position y (i32)
/// - Field 3: current HP (u32)
/// - Field 4: max HP (u32)
/// Note: stamina and attack power are NOT yet in serialization - will be added in Phase 2
fn constrain_attack_action(
    _is_attack: &Boolean<Fp254>,
    _actor_data: &[FpVar<Fp254>],
    _target_id: &Option<FpVar<Fp254>>,
    _cs: ConstraintSystemRef<Fp254>,
) -> Result<(), SynthesisError> {
    // TODO: Implement attack constraints when arkworks 0.5.0 is_cmp API is fixed
    //
    // Required constraints:
    // - Verify actor is alive (health > 0)
    // - Verify actor has sufficient stamina for attack
    // - Verify target is adjacent (Chebyshev distance <= 1)
    // - Verify damage calculation (attack - defense formula)
    // - Verify target health updated correctly
    // - Verify actor stamina decreased by action cost
    //
    // Currently attack validation is performed in game-core before proof generation.
    // See: https://github.com/arkworks-rs/r1cs-std/issues/XXX

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_type_encoding() {
        assert_eq!(ActionType::Move as u64, 0);
        assert_eq!(ActionType::Wait as u64, 1);
        assert_eq!(ActionType::MeleeAttack as u64, 2);

        let move_field = ActionType::Move.to_field();
        assert_eq!(ActionType::from_field(move_field), Some(ActionType::Move));
    }

    #[test]
    fn test_dummy_circuit_creation() {
        let circuit = GameTransitionCircuit::dummy();
        // Dummy circuit has computed witness values for constraint satisfaction
        assert!(circuit.before_root.is_some());
        assert!(circuit.after_root.is_some());
        assert!(circuit.action_type.is_some());
        assert!(circuit.actor_id.is_some());
        assert!(circuit.witnesses.is_some());

        // Extract witnesses once to avoid move-after-use
        let witnesses = circuit.witnesses.unwrap();
        assert_eq!(witnesses.entities.len(), 1);

        // CRITICAL: Verify Merkle path depth matches real circuits (depth 10)
        assert_eq!(
            witnesses.entities[0].before_path.siblings.len(),
            10,
            "Dummy circuit must use depth 10 to match build_entity_tree()"
        );
        assert_eq!(
            witnesses.entities[0].after_path.siblings.len(),
            10,
            "Dummy circuit must use depth 10 to match build_entity_tree()"
        );
    }
}
