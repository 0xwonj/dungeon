//! Integration test for ArkworksProver cryptographic verification.
//!
//! Tests that the Prover trait's verify() method correctly performs
//! cryptographic verification using Groth16.

#![cfg(feature = "arkworks")]

use game_core::*;
use zk::circuit::test_helpers::create_test_state_with_enemy;
use zk::circuit::ArkworksProver;
use zk::Prover;

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
#[ignore] // This test is slow (~5 seconds) - run manually with --ignored
fn test_verify_valid_proof() {
    // Create test states for a Move action (which we know works)
    let before_state = create_simple_test_state();
    let mut after_state = before_state.clone();
    after_state.entities.actors[0].position = Position::new(5, 6); // Move north
    after_state.turn.nonce = 1;

    let action = Action::Character(CharacterAction {
        actor: EntityId::PLAYER,
        kind: ActionKind::Move,
        input: ActionInput::Direction(CardinalDirection::North),
    });

    // Create prover with cached keys (required for verification)
    let prover = ArkworksProver::with_cached_keys()
        .expect("Failed to create prover with cached keys");

    // Generate proof
    let proof_data = prover
        .prove(&before_state, &action, &after_state)
        .expect("Failed to generate proof");

    // Verify proof should succeed
    let is_valid = prover.verify(&proof_data).expect("Verification failed");

    assert!(
        is_valid,
        "Valid proof should verify successfully. Proof data: backend={:?}, proof_bytes={}, public_inputs_count={}",
        proof_data.backend,
        proof_data.bytes.len(),
        proof_data.public_inputs.as_ref().map(|pi| pi.len()).unwrap_or(0)
    );
}

#[test]
fn test_verify_requires_cached_keys() {
    // Create test states
    let before_state = create_simple_test_state();
    let mut after_state = before_state.clone();
    after_state.entities.actors[0].position = Position::new(5, 6);

    let action = Action::Character(CharacterAction {
        actor: EntityId::PLAYER,
        kind: ActionKind::Wait,
        input: ActionInput::None,
    });

    // Create prover WITHOUT cached keys
    let prover_no_keys = ArkworksProver::new();

    // Create prover WITH cached keys to generate a proof
    let prover_with_keys = ArkworksProver::with_cached_keys()
        .expect("Failed to create prover with cached keys");

    let proof_data = prover_with_keys
        .prove(&before_state, &action, &after_state)
        .expect("Failed to generate proof");

    // Verify should fail without cached keys
    let result = prover_no_keys.verify(&proof_data);

    assert!(
        result.is_err(),
        "Verification without cached keys should fail"
    );
    assert!(
        result.unwrap_err().to_string().contains("cached keys"),
        "Error should mention missing cached keys"
    );
}

#[test]
#[ignore] // This test is slow (~5 seconds) - run manually with --ignored
fn test_verify_requires_public_inputs() {
    // Create prover with cached keys
    let prover = ArkworksProver::with_cached_keys()
        .expect("Failed to create prover with cached keys");

    // Create proof data without public inputs
    // Note: We need a valid proof structure, so we'll generate a real proof
    // then remove the public inputs
    let before_state = create_simple_test_state();
    let mut after_state = before_state.clone();
    after_state.entities.actors[0].position = Position::new(5, 6);
    after_state.turn.nonce = 1;

    let action = Action::Character(CharacterAction {
        actor: EntityId::PLAYER,
        kind: ActionKind::Move,
        input: ActionInput::Direction(CardinalDirection::North),
    });

    let mut proof_data = prover
        .prove(&before_state, &action, &after_state)
        .expect("Failed to generate proof");

    // Remove public inputs
    proof_data.public_inputs = None;

    // Verify should fail
    let result = prover.verify(&proof_data);

    assert!(
        result.is_err(),
        "Verification without public inputs should fail"
    );
    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("missing public inputs"),
        "Error should mention missing public inputs. Got: {}",
        error_msg
    );
}

#[test]
fn test_proof_contains_public_inputs() {
    // Create test states
    let before_state = create_simple_test_state();
    let mut after_state = before_state.clone();
    after_state.entities.actors[0].position = Position::new(5, 6);
    after_state.turn.nonce = 1;

    let action = Action::Character(CharacterAction {
        actor: EntityId::PLAYER,
        kind: ActionKind::Move,
        input: ActionInput::Direction(CardinalDirection::North),
    });

    // Create prover with cached keys
    let prover = ArkworksProver::with_cached_keys()
        .expect("Failed to create prover with cached keys");

    // Generate proof
    let proof_data = prover
        .prove(&before_state, &action, &after_state)
        .expect("Failed to generate proof");

    // Verify proof contains public inputs
    assert!(
        proof_data.public_inputs.is_some(),
        "Proof should contain public inputs"
    );

    let public_inputs = proof_data.public_inputs.unwrap();
    assert_eq!(
        public_inputs.len(),
        4,
        "Should have 4 public inputs: [before_root, after_root, action_type, actor_id]"
    );
}
