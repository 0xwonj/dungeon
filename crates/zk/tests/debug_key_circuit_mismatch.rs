//! Test to check if there's a mismatch between key generation circuit and proving circuit

#![cfg(feature = "arkworks")]

use ark_bn254::Fr as Fp254;
use ark_std::test_rng;
use game_core::{
    Action, ActionInput, ActionKind, CardinalDirection, CharacterAction, EntityId, Position, TurnState,
};
use zk::circuit::game_transition::{ActionType, GameTransitionCircuit};
use zk::circuit::test_helpers::create_test_state_with_enemy;
use zk::circuit::{groth16, merkle, witness};

#[test]
#[ignore]
fn test_key_generation_vs_proving_circuit() {
    let mut rng = test_rng();

    // Step 1: Generate keys with dummy circuit
    println!("=== Step 1: Key Generation ===");
    let dummy = GameTransitionCircuit::dummy();
    println!("Dummy circuit:");
    println!("  before_root: {}", dummy.before_root.unwrap());
    println!("  after_root: {}", dummy.after_root.unwrap());
    println!("  action_type: {}", dummy.action_type.unwrap());
    println!("  actor_id: {}", dummy.actor_id.unwrap());

    let keys = groth16::Groth16Keys::generate(dummy, &mut rng).expect("Key generation failed");
    println!("✓ Keys generated");

    // Step 2: Create real circuit
    println!("\n=== Step 2: Real Circuit ===");
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

    println!("Real circuit:");
    println!("  before_root: {}", before_root);
    println!("  after_root: {}", after_root);
    println!("  action_type: {}", ActionType::Move.to_field());
    println!("  actor_id: {}", Fp254::from(EntityId::PLAYER.0 as u64));
    println!("  witnesses: {}", witnesses.entities.len());
    println!("  witness path depth: {}", witnesses.entities[0].before_path.siblings.len());

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

    // Step 3: Generate proof
    println!("\n=== Step 3: Proof Generation ===");
    let proof = groth16::prove(circuit, &keys, &mut rng).expect("Proof generation failed");
    println!("✓ Proof generated");

    // Step 4: Verify
    println!("\n=== Step 4: Verification ===");
    let public_inputs = vec![
        before_root,
        after_root,
        ActionType::Move.to_field(),
        Fp254::from(EntityId::PLAYER.0 as u64),
    ];

    println!("Public inputs:");
    for (i, input) in public_inputs.iter().enumerate() {
        println!("  [{}]: {}", i, input);
    }

    let is_valid = groth16::verify(&proof, &public_inputs, &keys.verifying_key)
        .expect("Verification failed");

    println!("\nVerification result: {}", is_valid);

    if !is_valid {
        println!("\n❌ VERIFICATION FAILED");
        println!("\nPossible causes:");
        println!("1. Circuit structure mismatch between dummy and real circuit");
        println!("2. Public inputs don't match what was proven");
        println!("3. Constraint system differences");
    } else {
        println!("\n✓ VERIFICATION SUCCEEDED!");
    }

    assert!(is_valid, "Proof should verify!");
}
