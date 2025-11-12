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
use game_core::{EntityId, GameState, StateDelta};

use super::merkle::{
    MerklePath, build_entity_tree, serialize_actor, serialize_item, serialize_prop,
};
use crate::ProofError;

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
    for actor_change in &delta.entities.actors.updated {
        let id = actor_change.id;

        // Find actor in before state
        let before_actor = before_state
            .entities
            .actors
            .iter()
            .find(|a| a.id == id)
            .ok_or_else(|| {
                ProofError::StateInconsistency(format!("Actor {} not found in before state", id.0))
            })?;

        // Find actor in after state
        let after_actor = after_state
            .entities
            .actors
            .iter()
            .find(|a| a.id == id)
            .ok_or_else(|| {
                ProofError::StateInconsistency(format!("Actor {} not found in after state", id.0))
            })?;

        // Serialize actor data to field elements
        let before_data = serialize_actor(before_actor).to_vec();
        let after_data = serialize_actor(after_actor).to_vec();

        // Generate Merkle paths in both trees
        let before_path = before_tree.prove(id.0)?;
        let after_path = after_tree.prove(id.0)?;

        // Package as witness
        entity_witnesses.push(EntityWitness {
            id,
            before_data,
            before_path,
            after_data,
            after_path,
        });
    }

    // Generate witnesses for changed props
    for prop_change in &delta.entities.props.updated {
        let id = prop_change.id;

        let before_prop = before_state
            .entities
            .props
            .iter()
            .find(|p| p.id == id)
            .ok_or_else(|| {
                ProofError::StateInconsistency(format!("Prop {} not found in before state", id.0))
            })?;

        let after_prop = after_state
            .entities
            .props
            .iter()
            .find(|p| p.id == id)
            .ok_or_else(|| {
                ProofError::StateInconsistency(format!("Prop {} not found in after state", id.0))
            })?;

        let before_data = serialize_prop(before_prop).to_vec();
        let after_data = serialize_prop(after_prop).to_vec();

        let before_path = before_tree.prove(id.0)?;
        let after_path = after_tree.prove(id.0)?;

        entity_witnesses.push(EntityWitness {
            id,
            before_data,
            before_path,
            after_data,
            after_path,
        });
    }

    // Generate witnesses for changed items
    for item_change in &delta.entities.items.updated {
        let id = item_change.id;

        let before_item = before_state
            .entities
            .items
            .iter()
            .find(|i| i.id == id)
            .ok_or_else(|| {
                ProofError::StateInconsistency(format!("Item {} not found in before state", id.0))
            })?;

        let after_item = after_state
            .entities
            .items
            .iter()
            .find(|i| i.id == id)
            .ok_or_else(|| {
                ProofError::StateInconsistency(format!("Item {} not found in after state", id.0))
            })?;

        let before_data = serialize_item(before_item).to_vec();
        let after_data = serialize_item(after_item).to_vec();

        let before_path = before_tree.prove(id.0)?;
        let after_path = after_tree.prove(id.0)?;

        entity_witnesses.push(EntityWitness {
            id,
            before_data,
            before_path,
            after_data,
            after_path,
        });
    }

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
        InventoryState, Position, TurnState, WorldState,
    };

    /// Helper to create a simple game state with one actor
    fn create_test_state(actor_position: Position) -> GameState {
        let actor = ActorState::new(
            EntityId::PLAYER,
            actor_position,
            CoreStats::default(),
            InventoryState::default(),
        );

        let entities = EntitiesState::new(
            unsafe { BoundedVec::from_vec_unchecked(vec![actor]) },
            BoundedVec::new(),
            BoundedVec::new(),
        );

        GameState::new(TurnState::default(), entities, WorldState::default())
    }

    #[test]
    fn test_generate_witnesses_simple_movement() {
        // Create before and after states with actor movement
        let before_state = create_test_state(Position::new(0, 0));
        let after_state = create_test_state(Position::new(1, 0));

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
        let before_state = create_test_state(Position::new(5, 5));
        let after_state = create_test_state(Position::new(5, 5));

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
        let before_state = create_test_state(Position::new(2, 3));
        let after_state = create_test_state(Position::new(2, 4));

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
