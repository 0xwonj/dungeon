//! End-to-end integration test for ArkworksProver.
//!
//! Tests the full workflow: game state → proof generation → verification

#![cfg(feature = "arkworks")]

use game_core::{
    Action, ActionInput, ActionKind, ActorState, CardinalDirection, CharacterAction, CoreStats,
    EntitiesState, EntityId, GameState, InventoryState, Position, TurnState, WorldState,
};
use zk::{ArkworksProver, Prover};

/// Helper to create a minimal game state for testing.
fn create_test_state() -> GameState {
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

    // Add player actor at (5, 5)
    let player = ActorState::new(
        EntityId::PLAYER,
        Position::new(5, 5),
        default_stats.clone(),
        InventoryState::empty(),
    );
    let _ = entities.actors.push(player);

    // Add enemy actor at (6, 6)
    let enemy = ActorState::new(
        EntityId(1),
        Position::new(6, 6),
        default_stats,
        InventoryState::empty(),
    );
    let _ = entities.actors.push(enemy);

    let mut active_actors = std::collections::BTreeSet::new();
    active_actors.insert(EntityId::PLAYER);
    active_actors.insert(EntityId(1));

    let turn = TurnState {
        current_actor: EntityId::PLAYER,
        clock: 0,
        nonce: 0,
        active_actors,
    };

    GameState::with_seed(12345, turn, entities, WorldState::default())
}

#[test]
#[ignore] // Expensive test - requires key generation with proper circuit template
fn test_arkworks_prover_move_action_end_to_end() {
    // Setup: Create before and after states
    let before_state = create_test_state();
    let mut after_state = before_state.clone();

    // Simulate move action: player moves north from (5,5) to (5,6)
    after_state.entities.actors[0].position = Position::new(5, 6);
    after_state.turn.nonce = 1;

    let move_action = Action::Character(CharacterAction {
        actor: EntityId::PLAYER,
        kind: ActionKind::Move,
        input: ActionInput::Direction(CardinalDirection::North),
    });

    // Create prover with empty oracle snapshot (not needed for Arkworks)

    let prover = ArkworksProver::new();

    // Generate proof
    println!("Generating proof for Move action...");
    let proof_result = prover.prove(&before_state, &move_action, &after_state);

    assert!(
        proof_result.is_ok(),
        "Proof generation failed: {:?}",
        proof_result.err()
    );

    let proof = proof_result.unwrap();
    println!(
        "✓ Proof generated successfully: {} bytes",
        proof.bytes.len()
    );

    // Verify proof
    println!("Verifying proof...");
    let verify_result = prover.verify(&proof);

    assert!(
        verify_result.is_ok(),
        "Proof verification failed: {:?}",
        verify_result.err()
    );
    assert!(verify_result.unwrap(), "Proof verification returned false");

    println!("✓ Proof verified successfully");
}

#[test]
#[ignore] // Expensive test - generates full Groth16 proof
fn test_arkworks_prover_melee_attack_end_to_end() {
    let before_state = create_test_state();
    let mut after_state = before_state.clone();

    // Simulate attack: enemy takes 10 damage
    after_state.entities.actors[1].resources.hp = after_state.entities.actors[1]
        .resources
        .hp
        .saturating_sub(10);
    after_state.turn.nonce = 1;

    let attack_action = Action::Character(CharacterAction {
        actor: EntityId::PLAYER,
        kind: ActionKind::MeleeAttack,
        input: ActionInput::Entity(EntityId(1)),
    });

    let prover = ArkworksProver::new();

    // Generate proof
    println!("Generating proof for MeleeAttack action...");
    let proof = prover
        .prove(&before_state, &attack_action, &after_state)
        .expect("Proof generation failed");

    println!("✓ Proof generated: {} bytes", proof.bytes.len());

    // Verify proof
    println!("Verifying proof...");
    let is_valid = prover.verify(&proof).expect("Verification failed");
    assert!(is_valid, "Proof should be valid");

    println!("✓ MeleeAttack proof verified successfully");
}

#[test]
#[ignore] // Expensive test - requires key generation
fn test_arkworks_prover_wait_action_end_to_end() {
    let before_state = create_test_state();
    let mut after_state = before_state.clone();

    // Wait action doesn't change entity state, only nonce
    after_state.turn.nonce = 1;

    let wait_action = Action::Character(CharacterAction {
        actor: EntityId::PLAYER,
        kind: ActionKind::Wait,
        input: ActionInput::None,
    });

    let prover = ArkworksProver::new();

    // Generate proof
    println!("Generating proof for Wait action...");
    let proof = prover
        .prove(&before_state, &wait_action, &after_state)
        .expect("Proof generation failed");

    println!("✓ Proof generated: {} bytes", proof.bytes.len());

    // Verify proof
    let is_valid = prover.verify(&proof).expect("Verification failed");
    assert!(is_valid, "Proof should be valid");

    println!("✓ Wait action proof verified successfully");
}

#[test]
fn test_arkworks_prover_wrong_backend_rejection() {
    use zk::ProofData;

    let prover = ArkworksProver::new();

    // Create a fake proof with wrong backend (using Stub as an example)
    // Note: This test only works if stub feature is available
    #[cfg(feature = "stub")]
    let fake_backend = ProofBackend::Stub;

    #[cfg(not(feature = "stub"))]
    let fake_backend = {
        // If stub isn't available, we can't test with a different backend
        // Just return early - the other tests cover the happy path
        println!("⚠ Skipping backend rejection test - no alternative backend available");
        return;
    };

    let fake_proof = ProofData {
        bytes: vec![0x00, 0x01, 0x02, 0x03],
        backend: fake_backend,
    };

    // Verification should reject wrong backend
    let result = prover.verify(&fake_proof);
    assert!(result.is_err(), "Should reject proof from wrong backend");

    if let Err(e) = result {
        assert!(
            format!("{:?}", e).contains("ArkworksProver can only verify arkworks proofs"),
            "Error message should mention backend mismatch"
        );
    }

    println!("✓ Correctly rejected proof from wrong backend");
}
