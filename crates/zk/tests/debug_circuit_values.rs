//! Debug test to print circuit witness values.

#![cfg(feature = "arkworks")]

use ark_bn254::Fr as Fp254;
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystem};
use game_core::{
    ActorState, Action, ActionInput, ActionKind, CardinalDirection, CharacterAction, CoreStats,
    EntityId, EntitiesState, GameState, InventoryState, Position, StateDelta, TurnState,
    WorldState,
};
use zk::circuit::{game_transition::GameTransitionCircuit, merkle, witness};

#[test]
fn test_debug_move_action_witness_values() {
    // Create before state: player at (5, 5)
    let mut entities = EntitiesState::empty();
    let default_stats = CoreStats {
        str: 10,
        con: 10,
        dex: 10,
        int: 10,
        wil: 10,
        ego: 10,
        level: 1,
    };

    let player = ActorState::new(
        EntityId::PLAYER,
        Position::new(5, 5),
        default_stats.clone(),
        InventoryState::empty(),
    );
    let _ = entities.actors.push(player);

    let mut active_actors = std::collections::BTreeSet::new();
    active_actors.insert(EntityId::PLAYER);

    let turn = TurnState {
        current_actor: EntityId::PLAYER,
        clock: 0,
        nonce: 0,
        active_actors,
    };

    let before_state = GameState::with_seed(12345, turn, entities.clone(), WorldState::default());

    // Create after state: player at (5, 6)
    let mut after_state = before_state.clone();
    after_state.entities.actors[0].position = Position::new(5, 6);
    after_state.turn.nonce = 1;

    let move_action = Action::Character(CharacterAction {
        actor: EntityId::PLAYER,
        kind: ActionKind::Move,
        input: ActionInput::Direction(CardinalDirection::North),
    });

    // Generate delta and witnesses
    let delta = StateDelta::from_states(move_action.clone(), &before_state, &after_state);
    let witnesses = witness::generate_witnesses(&delta, &before_state, &after_state)
        .expect("Witness generation failed");

    println!("=== Witness Debug Info ===");
    println!("Number of entity witnesses: {}", witnesses.entities.len());

    if let Some(witness) = witnesses.entities.first() {
        println!("\nActor witness (EntityId {}):", witness.id.0);
        println!("  Before data ({} fields):", witness.before_data.len());
        for (i, field) in witness.before_data.iter().enumerate() {
            println!("    Field {}: {:?}", i, field);
        }

        println!("\n  After data ({} fields):", witness.after_data.len());
        for (i, field) in witness.after_data.iter().enumerate() {
            println!("    Field {}: {:?}", i, field);
        }

        println!("\n  Before path: {} siblings", witness.before_path.siblings.len());
        println!("  After path: {} siblings", witness.after_path.siblings.len());
    }

    // Compute entity tree roots (MVP: no turn state in circuit yet)
    let mut before_tree = merkle::build_entity_tree(&before_state).expect("Failed to build before tree");
    let before_root = before_tree.root().expect("Failed to compute before root");

    let mut after_tree = merkle::build_entity_tree(&after_state).expect("Failed to build after tree");
    let after_root = after_tree.root().expect("Failed to compute after root");

    println!("\n=== State Roots ===");
    println!("Before root: {:?}", before_root);
    println!("After root: {:?}", after_root);

    // Extract action parameters
    let actor_id = Fp254::from(EntityId::PLAYER.0 as u64);
    let action_type = Fp254::from(0u64); // Move = 0

    // Calculate position delta
    let dx = after_state.entities.actors[0].position.x - before_state.entities.actors[0].position.x;
    let dy = after_state.entities.actors[0].position.y - before_state.entities.actors[0].position.y;
    let position_delta = (Fp254::from(dx as i64), Fp254::from(dy as i64));

    println!("\n=== Action Parameters ===");
    println!("Action type (Move): {:?}", action_type);
    println!("Actor ID: {:?}", actor_id);
    println!("Position delta: dx={}, dy={}", dx, dy);
    println!("Position delta as Fp254: ({:?}, {:?})", position_delta.0, position_delta.1);

    // Create circuit
    let circuit = GameTransitionCircuit::new(
        before_root,
        after_root,
        action_type,
        actor_id,
        witnesses,
        None,
        None,
        Some(position_delta),
    );

    // Try to generate constraints
    let cs = ConstraintSystem::<Fp254>::new_ref();
    println!("\n=== Constraint Generation ===");
    match circuit.generate_constraints(cs.clone()) {
        Ok(()) => {
            println!("✓ Constraints generated successfully");
            println!("  Number of constraints: {}", cs.num_constraints());
            println!("  Constraints satisfied: {}", cs.is_satisfied().unwrap());

            if !cs.is_satisfied().unwrap() {
                println!("\n⚠ CONSTRAINTS NOT SATISFIED!");
                println!("This means some constraint failed during witness assignment.");
            }
        }
        Err(e) => {
            println!("✗ Constraint generation failed: {:?}", e);
        }
    }
}
