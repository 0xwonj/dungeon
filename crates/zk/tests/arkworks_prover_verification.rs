//! Integration test for ArkworksProver cryptographic verification.
//!
//! Tests that the Prover trait's verify() method correctly performs
//! cryptographic verification using Groth16.

#![cfg(feature = "arkworks")]

use game_core::*;
use zk::Prover;
use zk::circuit::ArkworksProver;
use zk::circuit::test_helpers::create_test_state_with_enemy;

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

    // Create prover without cached keys (generates keys per-proof with matching circuit structure)
    let prover = ArkworksProver::new();

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
        proof_data
            .public_inputs
            .as_ref()
            .map(|pi| pi.len())
            .unwrap_or(0)
    );
}

#[test]
fn test_verify_requires_verifying_key() {
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

    // Create prover and generate proof
    let prover = ArkworksProver::new();

    let mut proof_data = prover
        .prove(&before_state, &action, &after_state)
        .expect("Failed to generate proof");

    // Remove verifying key from proof data
    proof_data.verifying_key = None;

    // Verify should fail without verifying key
    let result = prover.verify(&proof_data);

    assert!(
        result.is_err(),
        "Verification without verifying key should fail"
    );
    assert!(
        result.unwrap_err().to_string().contains("verifying key"),
        "Error should mention missing verifying key"
    );
}

#[test]
fn test_verify_requires_public_inputs() {
    // Create prover
    let prover = ArkworksProver::new();

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

    // Create prover
    let prover = ArkworksProver::new();

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
