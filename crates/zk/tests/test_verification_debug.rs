//! Debug test to understand why verification fails

#![cfg(feature = "arkworks")]

use ark_bn254::Fr as Fp254;
use ark_std::test_rng;
use game_core::*;
use zk::Prover;
use zk::circuit::ArkworksProver;
use zk::circuit::test_helpers::create_test_state_with_enemy;
use zk::circuit::{game_transition::*, groth16, merkle, witness};

fn create_simple_test_state() -> GameState {
    let mut state = create_test_state_with_enemy(false);
    let mut active_actors = std::collections::BTreeSet::new();
    active_actors.insert(EntityId::PLAYER);
    state.turn = TurnState {
        current_actor: EntityId::PLAYER,
        clock: 0,
        nonce: 0,
        active_actors,
    };
    state
}

#[test]
fn test_direct_groth16_verification() {
    println!("\n=== Testing direct groth16::verify() ===");

    let before_state = create_simple_test_state();
    let mut after_state = before_state.clone();
    after_state.entities.actors[0].position = Position::new(5, 6);
    after_state.turn.nonce = 1;

    let action = Action::Character(CharacterAction {
        actor: EntityId::PLAYER,
        kind: ActionKind::Move,
        input: ActionInput::Direction(CardinalDirection::North),
    });

    // Build circuit manually
    let delta = game_core::StateDelta::from_states(action.clone(), &before_state, &after_state);

    let mut before_tree = merkle::build_entity_tree(&before_state).unwrap();
    let before_root = before_tree.root().unwrap();

    let mut after_tree = merkle::build_entity_tree(&after_state).unwrap();
    let after_root = after_tree.root().unwrap();

    let witnesses = witness::generate_witnesses(&delta, &before_state, &after_state).unwrap();

    let action_type = ActionType::Move.to_field();
    let actor_id = Fp254::from(EntityId::PLAYER.0 as u64);

    let circuit = GameTransitionCircuit::new(
        before_root,
        after_root,
        action_type,
        actor_id,
        witnesses.clone(),
        None,
        Some(Fp254::from(0u64)),                      // North
        Some((Fp254::from(0i64), Fp254::from(1i64))), // delta
    );

    let mut rng = test_rng();
    // CRITICAL: Use same circuit structure for key generation
    let key_gen_circuit = GameTransitionCircuit::new(
        before_root,
        after_root,
        action_type,
        actor_id,
        witnesses,
        None,
        Some(Fp254::from(0u64)),
        Some((Fp254::from(0i64), Fp254::from(1i64))),
    );
    let keys = groth16::Groth16Keys::generate(key_gen_circuit, &mut rng).unwrap();

    println!("Generating proof...");
    let proof = groth16::prove(circuit, &keys, &mut rng).unwrap();

    let public_inputs = vec![before_root, after_root, action_type, actor_id];

    println!("Public inputs:");
    for (i, inp) in public_inputs.iter().enumerate() {
        println!("  [{}] = {:?}", i, inp);
    }

    println!("Verifying with groth16::verify()...");
    let is_valid = groth16::verify(&proof, &public_inputs, &keys.verifying_key).unwrap();

    println!("Direct groth16::verify() result: {}", is_valid);
    assert!(is_valid, "Direct groth16 verification should succeed");
}

#[test]
fn test_prover_trait_verification() {
    println!("\n=== Testing Prover::verify() ===");

    let before_state = create_simple_test_state();
    let mut after_state = before_state.clone();
    after_state.entities.actors[0].position = Position::new(5, 6);
    after_state.turn.nonce = 1;

    let action = Action::Character(CharacterAction {
        actor: EntityId::PLAYER,
        kind: ActionKind::Move,
        input: ActionInput::Direction(CardinalDirection::North),
    });

    // Use new() instead of with_cached_keys() because cached keys only work for 1-entity circuits
    let prover = ArkworksProver::new();

    println!("Generating proof via Prover::prove()...");
    let proof_data = prover.prove(&before_state, &action, &after_state).unwrap();

    println!("Proof data:");
    println!("  bytes length: {}", proof_data.bytes.len());
    println!("  backend: {:?}", proof_data.backend);
    println!(
        "  public_inputs: {:?}",
        proof_data.public_inputs.as_ref().map(|pi| pi.len())
    );
    println!(
        "  verifying_key: {:?}",
        proof_data.verifying_key.as_ref().map(|vk| vk.len())
    );

    if let Some(ref public_inputs) = proof_data.public_inputs {
        use ark_serialize::CanonicalDeserialize;
        println!("  Deserialized public inputs:");
        for (i, bytes) in public_inputs.iter().enumerate() {
            let field = Fp254::deserialize_compressed(bytes.as_slice()).unwrap();
            println!("    [{}] = {:?}", i, field);
        }
    }

    println!("Verifying with Prover::verify()...");
    let result = prover.verify(&proof_data);
    println!("Verification result: {:?}", result);
    let is_valid = result.unwrap();

    println!("Prover::verify() result: {}", is_valid);
    assert!(is_valid, "Prover verification should succeed");
}
