//! Witness generation from StateDelta.
//!
//! Converts game state deltas into ZK circuit witnesses with Merkle proofs.
//!
//! This module implements the core algorithm for generating Merkle witnesses from StateDelta:
//! 1. Build Merkle trees from before and after states using batch operations
//! 2. For each changed entity in the delta, generate before/after witnesses
//! 3. Package witnesses for circuit consumption
//!
//! OPTIMIZATION: Uses batch tree building and pre-allocated vectors.
//!
//! See: docs/state-delta-architecture.md Section 5.4

use ark_bn254::Fr as Fp254;
use game_core::{ActorState, EntityId, GameState, ItemState, PropState, StateDelta};

use super::merkle::{
    MerklePath, SparseMerkleTree, build_entity_tree, serialize_actor, serialize_item,
    serialize_prop,
};
use crate::ProofError;

// ============================================================================
// Entity ID Trait for Generic Witness Generation
// ============================================================================

/// Trait for entities that have an EntityId.
///
/// This enables generic witness generation across different entity types.
trait HasEntityId {
    fn entity_id(&self) -> EntityId;
}

impl HasEntityId for ActorState {
    fn entity_id(&self) -> EntityId {
        self.id
    }
}

impl HasEntityId for PropState {
    fn entity_id(&self) -> EntityId {
        self.id
    }
}

impl HasEntityId for ItemState {
    fn entity_id(&self) -> EntityId {
        self.id
    }
}

// ============================================================================
// Generic Witness Generation Helper
// ============================================================================

/// Generate witnesses for a collection of changed entities.
///
/// This generic helper eliminates code duplication across actor/prop/item witness generation.
///
/// # Type Parameters
/// * `T` - Entity type (must implement HasEntityId)
/// * `F` - Serialization function type
///
/// # Arguments
/// * `changed_ids` - Iterator of changed entity IDs from delta
/// * `before_entities` - Slice of entities in before state
/// * `after_entities` - Slice of entities in after state
/// * `before_tree` - Merkle tree for before state
/// * `after_tree` - Merkle tree for after state
/// * `serialize_fn` - Function to serialize entity to field elements
/// * `entity_type` - Human-readable entity type name for error messages
///
/// # Returns
/// Vector of EntityWitness structs for all changed entities
#[inline]
fn generate_entity_witnesses<'a, T, I, F>(
    changed_ids: I,
    before_entities: &'a [T],
    after_entities: &'a [T],
    before_tree: &mut SparseMerkleTree,
    after_tree: &mut SparseMerkleTree,
    serialize_fn: F,
    entity_type: &str,
) -> Result<Vec<EntityWitness>, ProofError>
where
    T: HasEntityId,
    I: Iterator<Item = EntityId>,
    F: Fn(&T) -> [Fp254; 5],
{
    changed_ids
        .map(|id| {
            // Find entity in before state
            let before_entity = before_entities
                .iter()
                .find(|e| e.entity_id() == id)
                .ok_or_else(|| {
                    ProofError::StateInconsistency(format!(
                        "{} {} not found in before state",
                        entity_type, id.0
                    ))
                })?;

            // Find entity in after state
            let after_entity = after_entities
                .iter()
                .find(|e| e.entity_id() == id)
                .ok_or_else(|| {
                    ProofError::StateInconsistency(format!(
                        "{} {} not found in after state",
                        entity_type, id.0
                    ))
                })?;

            // Serialize entity data
            let before_data = serialize_fn(before_entity).to_vec();
            let after_data = serialize_fn(after_entity).to_vec();

            // Generate Merkle paths
            let before_path = before_tree.prove(id.0)?;
            let after_path = after_tree.prove(id.0)?;

            // Package as witness
            Ok(EntityWitness {
                id,
                before_data,
                before_path,
                after_data,
                after_path,
            })
        })
        .collect()
}

/// Witness for a single entity change.
///
/// Contains Merkle authentication paths for both before and after states,
/// along with the serialized entity data.
///
/// # Fields
///
/// - `id`: Entity identifier
/// - `before_data`: Serialized entity fields from before state
/// - `before_path`: Merkle path proving entity in before tree
/// - `after_data`: Serialized entity fields from after state
/// - `after_path`: Merkle path proving entity in after tree
#[derive(Clone, Debug)]
pub struct EntityWitness {
    pub id: EntityId,
    pub before_data: Vec<Fp254>,
    pub before_path: MerklePath,
    pub after_data: Vec<Fp254>,
    pub after_path: MerklePath,
}

/// All witnesses for a state transition.
///
/// Contains witnesses for all changed entities (actors, props, items)
/// during a single action execution.
///
/// # Design Notes
///
/// This structure is optimized for circuit consumption:
/// - Only changed entities have witnesses (sparse representation)
/// - Witnesses are ordered by entity ID for deterministic circuit layout
/// - Both before/after paths included for state transition verification
#[derive(Clone, Debug)]
pub struct TransitionWitnesses {
    pub entities: Vec<EntityWitness>,
}

/// Generate witnesses from StateDelta.
///
/// This is the main entry point for witness generation. It analyzes the delta
/// to determine which entities changed, then generates Merkle witnesses for
/// those entities only.
///
/// # Algorithm
///
/// 1. Build Merkle trees from before_state and after_state
/// 2. For each changed actor in delta:
///    a. Find actor in before/after states
///    b. Serialize actor fields to field elements
///    c. Generate Merkle paths in both trees
///    d. Package as EntityWitness
/// 3. Repeat for changed props and items
/// 4. Return complete witness set
///
/// # Complexity
///
/// - Time: O(n log n + k log n) where n = entity count, k = changed entities
/// - Space: O(k log n) for witness storage
///
/// # Arguments
///
/// * `delta` - StateDelta describing which entities changed
/// * `before_state` - Game state before action execution
/// * `after_state` - Game state after action execution
///
/// # Returns
///
/// * `Ok(TransitionWitnesses)` - Witnesses for all changed entities
/// * `Err(ProofError)` - If witness generation fails (missing entity, tree error, etc.)
///
/// # Errors
///
/// - `StateInconsistency`: Entity in delta not found in before/after state
/// - `MerkleTreeError`: Merkle tree construction or proof generation failed
pub fn generate_witnesses(
    delta: &StateDelta,
    before_state: &GameState,
    after_state: &GameState,
) -> Result<TransitionWitnesses, ProofError> {
    let mut entity_witnesses = Vec::new();

    // Build Merkle trees for before and after states
    // These trees contain all entities (actors, props, items) indexed by entity ID
    let mut before_tree = build_entity_tree(before_state)?;
    let mut after_tree = build_entity_tree(after_state)?;

    // Generate witnesses for changed actors
    entity_witnesses.extend(generate_entity_witnesses(
        delta.entities.actors.updated.iter().map(|c| c.id),
        &before_state.entities.actors,
        &after_state.entities.actors,
        &mut before_tree,
        &mut after_tree,
        serialize_actor,
        "Actor",
    )?);

    // Generate witnesses for changed props
    entity_witnesses.extend(generate_entity_witnesses(
        delta.entities.props.updated.iter().map(|c| c.id),
        &before_state.entities.props,
        &after_state.entities.props,
        &mut before_tree,
        &mut after_tree,
        serialize_prop,
        "Prop",
    )?);

    // Generate witnesses for changed items
    entity_witnesses.extend(generate_entity_witnesses(
        delta.entities.items.updated.iter().map(|c| c.id),
        &before_state.entities.items,
        &after_state.entities.items,
        &mut before_tree,
        &mut after_tree,
        serialize_item,
        "Item",
    )?);


    // Sort witnesses by entity ID for deterministic circuit layout
    entity_witnesses.sort_by_key(|w| w.id.0);

    Ok(TransitionWitnesses {
        entities: entity_witnesses,
    })
}

#[cfg(test)]
mod tests {
    use super::super::merkle::{SparseMerkleTree, hash_many};
    use super::*;
    use bounded_vector::BoundedVec;
    use game_core::{
        Action, ActionInput, ActionKind, ActorState, CharacterAction, CoreStats, EntitiesState,
        EntityId, InventoryState, Position, StateDelta, TurnState, WorldState,
    };

    use crate::circuit::test_helpers::create_test_state_at_position;


    #[test]
    fn test_generate_witnesses_simple_movement() {
        // Create before and after states with actor movement
        let before_state = create_test_state_at_position(Position::new(0, 0));
        let after_state = create_test_state_at_position(Position::new(1, 0));

        // Create delta
        let action = Action::Character(CharacterAction::new(
            EntityId::PLAYER,
            ActionKind::Wait,
            ActionInput::None,
        ));
        let delta = StateDelta::from_states(action, &before_state, &after_state);

        // Generate witnesses
        let witnesses = generate_witnesses(&delta, &before_state, &after_state)
            .expect("Witness generation should succeed");

        // Verify we got a witness for the changed actor
        assert_eq!(
            witnesses.entities.len(),
            1,
            "Should have one entity witness"
        );

        let witness = &witnesses.entities[0];
        assert_eq!(witness.id, EntityId::PLAYER, "Witness should be for player");

        // Verify before/after data is different (position changed)
        assert_ne!(
            witness.before_data, witness.after_data,
            "Before and after data should differ"
        );

        // Verify Merkle paths exist and have correct depth
        assert_eq!(
            witness.before_path.siblings.len(),
            10,
            "Merkle path should have depth 10"
        );
        assert_eq!(
            witness.after_path.siblings.len(),
            10,
            "Merkle path should have depth 10"
        );
    }

    #[test]
    fn test_generate_witnesses_no_changes() {
        // Create identical before and after states
        let before_state = create_test_state_at_position(Position::new(5, 5));
        let after_state = create_test_state_at_position(Position::new(5, 5));

        let action = Action::Character(CharacterAction::new(
            EntityId::PLAYER,
            ActionKind::Wait,
            ActionInput::None,
        ));
        let delta = StateDelta::from_states(action, &before_state, &after_state);

        // Generate witnesses
        let witnesses = generate_witnesses(&delta, &before_state, &after_state)
            .expect("Witness generation should succeed");

        // No changes should result in no witnesses
        assert_eq!(
            witnesses.entities.len(),
            0,
            "No changes should produce no witnesses"
        );
    }

    #[test]
    fn test_generate_witnesses_multiple_actors() {
        // Create state with multiple actors, where only one changes
        let actor1 = ActorState::new(
            EntityId::PLAYER,
            Position::new(0, 0),
            CoreStats::default(),
            InventoryState::default(),
        );
        let actor2 = ActorState::new(
            EntityId(1),
            Position::new(5, 5),
            CoreStats::default(),
            InventoryState::default(),
        );

        let entities_before = EntitiesState::new(
            unsafe { BoundedVec::from_vec_unchecked(vec![actor1.clone(), actor2.clone()]) },
            BoundedVec::new(),
            BoundedVec::new(),
        );
        let before_state =
            GameState::new(TurnState::default(), entities_before, WorldState::default());

        // After state: only actor1 moves
        let actor1_moved = ActorState::new(
            EntityId::PLAYER,
            Position::new(1, 0),
            CoreStats::default(),
            InventoryState::default(),
        );

        let entities_after = EntitiesState::new(
            unsafe { BoundedVec::from_vec_unchecked(vec![actor1_moved, actor2]) },
            BoundedVec::new(),
            BoundedVec::new(),
        );
        let after_state =
            GameState::new(TurnState::default(), entities_after, WorldState::default());

        let action = Action::Character(CharacterAction::new(
            EntityId::PLAYER,
            ActionKind::Wait,
            ActionInput::None,
        ));
        let delta = StateDelta::from_states(action, &before_state, &after_state);

        // Generate witnesses
        let witnesses = generate_witnesses(&delta, &before_state, &after_state)
            .expect("Witness generation should succeed");

        // Should only have witness for changed actor
        assert_eq!(
            witnesses.entities.len(),
            1,
            "Should have witness only for changed actor"
        );
        assert_eq!(
            witnesses.entities[0].id,
            EntityId::PLAYER,
            "Witness should be for player"
        );
    }

    #[test]
    fn test_witness_verification_integrity() {
        // Create states with movement
        let before_state = create_test_state_at_position(Position::new(2, 3));
        let after_state = create_test_state_at_position(Position::new(2, 4));

        let action = Action::Character(CharacterAction::new(
            EntityId::PLAYER,
            ActionKind::Wait,
            ActionInput::None,
        ));
        let delta = StateDelta::from_states(action, &before_state, &after_state);

        // Generate witnesses
        let witnesses = generate_witnesses(&delta, &before_state, &after_state)
            .expect("Witness generation should succeed");

        assert_eq!(witnesses.entities.len(), 1);
        let witness = &witnesses.entities[0];

        // Build trees to verify paths
        let mut before_tree = build_entity_tree(&before_state).unwrap();
        let mut after_tree = build_entity_tree(&after_state).unwrap();

        let before_root = before_tree.root().unwrap();
        let after_root = after_tree.root().unwrap();

        // Verify before path
        let before_leaf_hash = hash_many(&witness.before_data).unwrap();
        let before_valid =
            SparseMerkleTree::verify(before_leaf_hash, &witness.before_path, before_root).unwrap();
        assert!(before_valid, "Before Merkle path should verify");

        // Verify after path
        let after_leaf_hash = hash_many(&witness.after_data).unwrap();
        let after_valid =
            SparseMerkleTree::verify(after_leaf_hash, &witness.after_path, after_root).unwrap();
        assert!(after_valid, "After Merkle path should verify");

        // Roots should be different (state changed)
        assert_ne!(before_root, after_root, "State roots should differ");
    }
}
