//! Integration tests for GameTransitionCircuit with real game actions.

#![cfg(feature = "arkworks")]

use ark_bn254::Fr as Fp254;
use ark_std::test_rng;
use game_core::{
    Action, ActionInput, ActionKind, CardinalDirection, CharacterAction, EntityId, GameState,
    Position, TurnState,
};
use zk::circuit::game_transition::{ActionType, GameTransitionCircuit};
use zk::circuit::test_helpers::create_test_state_with_enemy;
use zk::circuit::{groth16, merkle, witness};

fn setup_test_state() -> GameState {
    let mut state = create_test_state_with_enemy(true);
    let mut active_actors = std::collections::BTreeSet::new();
    active_actors.insert(EntityId::PLAYER);
    active_actors.insert(EntityId(1));
    state.turn = TurnState {
        current_actor: EntityId::PLAYER,
        clock: 0,
        nonce: 0,
        active_actors,
    };
    state
}

fn build_entity_roots(
    before: &GameState,
    after: &GameState,
) -> Result<(Fp254, Fp254), Box<dyn std::error::Error>> {
    let mut before_tree = merkle::build_entity_tree(before)?;
    let mut after_tree = merkle::build_entity_tree(after)?;
    Ok((before_tree.root()?, after_tree.root()?))
}

#[test]
fn test_move_action_witness_generation() {
    let before_state = setup_test_state();
    let action = Action::Character(CharacterAction {
        actor: EntityId::PLAYER,
        kind: ActionKind::Move,
        input: ActionInput::Direction(CardinalDirection::North),
    });

    let mut after_state = before_state.clone();
    after_state.entities.actors[0].position = Position::new(5, 6);
    after_state.turn.nonce = 1;

    let delta = game_core::StateDelta::from_states(action.clone(), &before_state, &after_state);
    let witnesses = witness::generate_witnesses(&delta, &before_state, &after_state).unwrap();

    assert!(!witnesses.entities.is_empty());
    assert_eq!(witnesses.entities[0].id, EntityId::PLAYER);
}

#[test]
fn test_move_action_state_roots() {
    let before_state = setup_test_state();
    let mut after_state = before_state.clone();
    after_state.entities.actors[0].position = Position::new(5, 6);

    let (before_root, after_root) = build_entity_roots(&before_state, &after_state).unwrap();
    assert_ne!(before_root, after_root);
}

#[test]
fn test_game_transition_circuit_construction() {
    let before_state = setup_test_state();
    let mut after_state = before_state.clone();
    after_state.entities.actors[0].position = Position::new(5, 6);

    let action = Action::Character(CharacterAction {
        actor: EntityId::PLAYER,
        kind: ActionKind::Move,
        input: ActionInput::Direction(CardinalDirection::North),
    });
    let delta = game_core::StateDelta::from_states(action, &before_state, &after_state);
    let (before_root, after_root) = build_entity_roots(&before_state, &after_state).unwrap();
    let witnesses = witness::generate_witnesses(&delta, &before_state, &after_state).unwrap();

    let _circuit = GameTransitionCircuit::new(
        before_root,
        after_root,
        ActionType::Move.to_field(),
        Fp254::from(EntityId::PLAYER.0 as u64),
        witnesses,
        None,
        Some(Fp254::from(0u64)),
        Some((Fp254::from(0i64), Fp254::from(1i64))),
    );
}

#[test]
#[ignore]
fn test_move_action_full_proof() {
    let before_state = setup_test_state();
    let mut after_state = before_state.clone();
    after_state.entities.actors[0].position = Position::new(5, 6);

    let action = Action::Character(CharacterAction {
        actor: EntityId::PLAYER,
        kind: ActionKind::Move,
        input: ActionInput::Direction(CardinalDirection::North),
    });
    let delta = game_core::StateDelta::from_states(action, &before_state, &after_state);
    let (before_root, after_root) = build_entity_roots(&before_state, &after_state).unwrap();
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

    let mut rng = test_rng();
    let dummy_circuit = GameTransitionCircuit::dummy();
    if let Ok(keys) = groth16::Groth16Keys::generate(dummy_circuit, &mut rng) {
        let proof = groth16::prove(circuit, &keys, &mut rng).unwrap();
        groth16::serialize_proof(&proof).unwrap();
    }
}

#[test]
#[ignore]
fn test_move_action_proof_verification() {
    let before_state = setup_test_state();
    let mut after_state = before_state.clone();
    after_state.entities.actors[0].position = Position::new(5, 6);
    after_state.turn.nonce = 1;

    let action = Action::Character(CharacterAction {
        actor: EntityId::PLAYER,
        kind: ActionKind::Move,
        input: ActionInput::Direction(CardinalDirection::North),
    });
    let delta = game_core::StateDelta::from_states(action, &before_state, &after_state);
    let (before_root, after_root) = build_entity_roots(&before_state, &after_state).unwrap();
    let witnesses = witness::generate_witnesses(&delta, &before_state, &after_state).unwrap();

    let circuit = GameTransitionCircuit::new(
        before_root,
        after_root,
        ActionType::Move.to_field(),
        Fp254::from(EntityId::PLAYER.0 as u64),
        witnesses.clone(),
        None,
        Some(Fp254::from(0u64)),
        Some((Fp254::from(0i64), Fp254::from(1i64))),
    );

    let mut rng = test_rng();
    let key_gen_circuit = GameTransitionCircuit::new(
        before_root,
        after_root,
        ActionType::Move.to_field(),
        Fp254::from(EntityId::PLAYER.0 as u64),
        witnesses,
        None,
        Some(Fp254::from(0u64)),
        Some((Fp254::from(0i64), Fp254::from(1i64))),
    );
    let keys = groth16::Groth16Keys::generate(key_gen_circuit, &mut rng).unwrap();
    let proof = groth16::prove(circuit, &keys, &mut rng).unwrap();

    let public_inputs = vec![
        before_root,
        after_root,
        ActionType::Move.to_field(),
        Fp254::from(EntityId::PLAYER.0 as u64),
    ];

    let is_valid = groth16::verify(&proof, &public_inputs, &keys.verifying_key).unwrap();
    assert!(is_valid);
}

#[test]
fn test_wait_action_witnesses() {
    let before_state = setup_test_state();
    let after_state = before_state.clone();

    let action = Action::Character(CharacterAction {
        actor: EntityId::PLAYER,
        kind: ActionKind::Wait,
        input: ActionInput::None,
    });
    let delta = game_core::StateDelta::from_states(action, &before_state, &after_state);
    witness::generate_witnesses(&delta, &before_state, &after_state).unwrap();
}

#[test]
fn test_melee_attack_action_witness_structure() {
    let before_state = setup_test_state();
    let mut after_state = before_state.clone();
    after_state.entities.actors[1].resources.hp = after_state.entities.actors[1]
        .resources
        .hp
        .saturating_sub(10);
    after_state.turn.nonce = 1;

    let action = Action::Character(CharacterAction {
        actor: EntityId::PLAYER,
        kind: ActionKind::MeleeAttack,
        input: ActionInput::Entity(EntityId(1)),
    });
    let delta = game_core::StateDelta::from_states(action, &before_state, &after_state);
    let witnesses = witness::generate_witnesses(&delta, &before_state, &after_state).unwrap();

    assert!(!witnesses.entities.is_empty());
}

