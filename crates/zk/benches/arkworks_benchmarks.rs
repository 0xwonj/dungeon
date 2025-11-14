//! Performance benchmarks for Arkworks circuit operations.
//!
//! Run with: cargo bench --package zk --features arkworks
//!
//! This will generate HTML reports in target/criterion/

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use game_core::{
    Action, ActionInput, ActionKind, CardinalDirection, CharacterAction, EntityId, GameState,
    Position, TurnState,
};

#[cfg(feature = "arkworks")]
use zk::circuit::test_helpers::create_test_state_with_actors;
#[cfg(feature = "arkworks")]
use zk::circuit::{merkle, witness};

/// Create a test game state with a given number of actors, plus active_actors set.
fn create_test_state(num_actors: usize) -> GameState {
    let mut state = create_test_state_with_actors(num_actors);

    // Set up turn state with active actors (just player for benchmarks)
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

#[cfg(feature = "arkworks")]
fn bench_merkle_tree_construction(c: &mut Criterion) {
    let mut group = c.benchmark_group("merkle_tree");

    for num_actors in [1, 5, 10, 20, 50].iter() {
        let state = create_test_state(*num_actors);

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_actors", num_actors)),
            num_actors,
            |b, _| {
                b.iter(|| {
                    let tree = merkle::build_entity_tree(black_box(&state));
                    black_box(tree)
                });
            },
        );
    }

    group.finish();
}

#[cfg(feature = "arkworks")]
fn bench_state_root_computation(c: &mut Criterion) {
    let mut group = c.benchmark_group("state_root");

    for num_actors in [1, 5, 10, 20, 50].iter() {
        let state = create_test_state(*num_actors);

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_actors", num_actors)),
            num_actors,
            |b, _| {
                b.iter(|| {
                    let root = merkle::compute_state_root(black_box(&state));
                    black_box(root)
                });
            },
        );
    }

    group.finish();
}

#[cfg(feature = "arkworks")]
fn bench_witness_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("witness_generation");

    for num_actors in [1, 5, 10].iter() {
        let before_state = create_test_state(*num_actors);
        let mut after_state = before_state.clone();

        // Modify first actor position (simulating a move)
        after_state.entities.actors[0].position = Position::new(6, 6);
        after_state.turn.nonce = 1;

        let action = Action::Character(CharacterAction {
            actor: EntityId::PLAYER,
            kind: ActionKind::Move,
            input: ActionInput::Direction(CardinalDirection::North),
        });

        let delta = game_core::StateDelta::from_states(action, &before_state, &after_state);

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_actors", num_actors)),
            num_actors,
            |b, _| {
                b.iter(|| {
                    let witnesses = witness::generate_witnesses(
                        black_box(&delta),
                        black_box(&before_state),
                        black_box(&after_state),
                    );
                    black_box(witnesses)
                });
            },
        );
    }

    group.finish();
}

#[cfg(feature = "arkworks")]
fn bench_state_transition_from_delta(c: &mut Criterion) {
    use zk::circuit::StateTransition;

    let mut group = c.benchmark_group("state_transition");

    for num_actors in [1, 5, 10].iter() {
        let before_state = create_test_state(*num_actors);
        let mut after_state = before_state.clone();
        after_state.entities.actors[0].position = Position::new(6, 6);

        let action = Action::Character(CharacterAction {
            actor: EntityId::PLAYER,
            kind: ActionKind::Move,
            input: ActionInput::Direction(CardinalDirection::North),
        });

        let delta = game_core::StateDelta::from_states(action, &before_state, &after_state);

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_actors", num_actors)),
            num_actors,
            |b, _| {
                b.iter(|| {
                    let transition = StateTransition::from_delta(
                        black_box(delta.clone()),
                        black_box(&before_state),
                        black_box(&after_state),
                    );
                    black_box(transition)
                });
            },
        );
    }

    group.finish();
}

#[cfg(feature = "arkworks")]
fn bench_poseidon_hash(c: &mut Criterion) {
    use ark_bn254::Fr as Fp254;
    use zk::circuit::commitment::{hash_one, hash_two};

    let mut group = c.benchmark_group("poseidon_hash");

    let value1 = Fp254::from(12345u64);
    let value2 = Fp254::from(67890u64);

    group.bench_function("hash_one", |b| {
        b.iter(|| {
            let result = hash_one(black_box(value1));
            black_box(result)
        });
    });

    group.bench_function("hash_two", |b| {
        b.iter(|| {
            let result = hash_two(black_box(value1), black_box(value2));
            black_box(result)
        });
    });

    group.finish();
}

#[cfg(feature = "arkworks")]
fn bench_merkle_proof_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("merkle_proof");

    for num_actors in [5, 10, 20].iter() {
        let state = create_test_state(*num_actors);
        let mut tree = merkle::build_entity_tree(&state).unwrap();

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_actors", num_actors)),
            num_actors,
            |b, _| {
                b.iter(|| {
                    let proof = tree.prove(black_box(0)); // Prove for player (ID 0)
                    black_box(proof)
                });
            },
        );
    }

    group.finish();
}

#[cfg(not(feature = "arkworks"))]
fn bench_placeholder(c: &mut Criterion) {
    c.bench_function("arkworks_disabled", |b| {
        b.iter(|| {
            // Placeholder when arkworks feature is not enabled
            println!("Arkworks benchmarks require --features arkworks");
        });
    });
}

#[cfg(feature = "arkworks")]
criterion_group!(
    benches,
    bench_merkle_tree_construction,
    bench_state_root_computation,
    bench_witness_generation,
    bench_state_transition_from_delta,
    bench_poseidon_hash,
    bench_merkle_proof_generation,
);

#[cfg(not(feature = "arkworks"))]
criterion_group!(benches, bench_placeholder);

criterion_main!(benches);
