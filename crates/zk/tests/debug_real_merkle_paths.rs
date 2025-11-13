//! Verify that real circuit's Merkle paths are valid

#![cfg(feature = "arkworks")]

use game_core::{
    Action, ActionInput, ActionKind, CardinalDirection, CharacterAction, EntityId, Position, TurnState,
};
use zk::circuit::test_helpers::create_test_state_with_enemy;
use zk::circuit::{merkle, witness};

#[test]
fn test_real_circuit_merkle_paths_validity() {
    // Create the exact same state as the failing test
    let mut state = create_test_state_with_enemy(false);
    let mut active_actors = std::collections::BTreeSet::new();
    active_actors.insert(EntityId::PLAYER);
    state.turn = TurnState {
        current_actor: EntityId::PLAYER,
        clock: 0,
        nonce: 0,
        active_actors,
    };

    let before_state = state;
    let mut after_state = before_state.clone();
    after_state.entities.actors[0].position = Position::new(5, 6);
    after_state.turn.nonce = 1;

    let move_action = Action::Character(CharacterAction {
        actor: EntityId::PLAYER,
        kind: ActionKind::Move,
        input: ActionInput::Direction(CardinalDirection::North),
    });

    let delta = game_core::StateDelta::from_states(move_action, &before_state, &after_state);
    let mut before_tree = merkle::build_entity_tree(&before_state).unwrap();
    let mut after_tree = merkle::build_entity_tree(&after_state).unwrap();
    let before_root = before_tree.root().unwrap();
    let after_root = after_tree.root().unwrap();
    let witnesses = witness::generate_witnesses(&delta, &before_state, &after_state).unwrap();

    println!("=== Merkle Path Validation ===");
    println!("Before root: {}", before_root);
    println!("After root: {}", after_root);

    // Verify each witness's Merkle paths
    for (i, w) in witnesses.entities.iter().enumerate() {
        println!("\n--- Witness {} (Entity ID {}) ---", i, w.id.0);

        // Compute leaf hashes
        let before_leaf = merkle::hash_many(&w.before_data).unwrap();
        let after_leaf = merkle::hash_many(&w.after_data).unwrap();

        println!("Before leaf hash: {}", before_leaf);
        println!("After leaf hash: {}", after_leaf);

        // Verify before path
        let before_valid = merkle::SparseMerkleTree::verify(
            before_leaf,
            &w.before_path,
            before_root,
        ).unwrap();

        println!("Before path valid: {}", before_valid);
        if !before_valid {
            println!("❌ BEFORE PATH INVALID!");
            println!("  Path siblings: {}", w.before_path.siblings.len());
            println!("  Path directions: {:?}", w.before_path.directions);
        }

        // Verify after path
        let after_valid = merkle::SparseMerkleTree::verify(
            after_leaf,
            &w.after_path,
            after_root,
        ).unwrap();

        println!("After path valid: {}", after_valid);
        if !after_valid {
            println!("❌ AFTER PATH INVALID!");
            println!("  Path siblings: {}", w.after_path.siblings.len());
            println!("  Path directions: {:?}", w.after_path.directions);
        }

        assert!(before_valid, "Before Merkle path must be valid");
        assert!(after_valid, "After Merkle path must be valid");
    }

    println!("\n✓ All Merkle paths are valid!");
}
