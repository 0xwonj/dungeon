//! Compare constraint counts between dummy and real circuits

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
fn test_constraint_count_match() {
    // Generate dummy circuit constraints
    let dummy = GameTransitionCircuit::dummy();
    let cs_dummy = ConstraintSystem::<Fp254>::new_ref();
    dummy.generate_constraints(cs_dummy.clone()).expect("Dummy constraint generation failed");
    let dummy_count = cs_dummy.num_constraints();

    println!("=== Dummy Circuit ===");
    println!("Constraints: {}", dummy_count);
    println!("Satisfied: {}", cs_dummy.is_satisfied().unwrap());

    // Generate real circuit constraints
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

    let cs_real = ConstraintSystem::<Fp254>::new_ref();
    circuit.generate_constraints(cs_real.clone()).expect("Real constraint generation failed");
    let real_count = cs_real.num_constraints();

    println!("\n=== Real Circuit ===");
    println!("Constraints: {}", real_count);
    println!("Satisfied: {}", cs_real.is_satisfied().unwrap());

    println!("\n=== Comparison ===");
    if dummy_count == real_count {
        println!("✓ Constraint counts MATCH ({} == {})", dummy_count, real_count);
    } else {
        println!("❌ Constraint counts MISMATCH ({} vs {})", dummy_count, real_count);
        println!("This means the circuit structure is different!");
    }

    assert_eq!(dummy_count, real_count, "Circuit structure must match!");
}
