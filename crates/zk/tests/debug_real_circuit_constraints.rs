//! Debug test to check if real circuit constraints are satisfied

#![cfg(feature = "arkworks")]

use ark_bn254::Fr as Fp254;
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystem};
use game_core::{
    Action, ActionInput, ActionKind, CardinalDirection, CharacterAction, EntityId, Position, TurnState,
};
use zk::circuit::game_transition::{ActionType, GameTransitionCircuit};
use zk::circuit::test_helpers::create_test_state_with_enemy;
use zk::circuit::{merkle, witness};

#[test]
fn test_real_circuit_constraint_satisfaction() {
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

    println!("=== Real Circuit Debug ===");
    println!("Before root: {}", before_root);
    println!("After root: {}", after_root);
    println!("Number of witnesses: {}", witnesses.entities.len());

    let circuit = GameTransitionCircuit::new(
        before_root,
        after_root,
        ActionType::Move.to_field(),
        Fp254::from(EntityId::PLAYER.0 as u64),
        witnesses,
        None,
        Some(Fp254::from(0u64)),
        Some((Fp254::from(0i64), Fp254::from(1i64))),
    );

    let cs = ConstraintSystem::<Fp254>::new_ref();
    println!("Generating constraints...");
    circuit.generate_constraints(cs.clone()).expect("Constraint generation failed");

    println!("Number of constraints: {}", cs.num_constraints());
    println!("Checking if constraints are satisfied...");

    let is_satisfied = cs.is_satisfied().unwrap();
    println!("Constraints satisfied: {}", is_satisfied);

    if !is_satisfied {
        println!("\n❌ REAL CIRCUIT CONSTRAINTS NOT SATISFIED!");
        println!("This is why proof verification fails!");
        println!("\nThe circuit is generating a proof for an unsatisfiable constraint system.");
        println!("Groth16 can generate proofs even with unsatisfied constraints,");
        println!("but verification will always fail.");
    } else {
        println!("\n✓ Real circuit constraints ARE satisfied");
        println!("The issue must be elsewhere (public input mismatch, etc.)");
    }

    assert!(is_satisfied, "Real circuit constraints must be satisfied for verification to succeed!");
}
