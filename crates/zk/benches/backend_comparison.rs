//! Cross-backend comparison benchmark for RISC0 vs Stub.
//!
//! This benchmark provides an apples-to-apples comparison of proof generation
//! and verification performance between backends.
//!
//! ## Usage
//!
//! 1. Run with RISC0 and save baseline:
//!    ```bash
//!    RISC0_SKIP_BUILD=1 cargo bench --package zk --no-default-features --features risc0 \
//!      --bench backend_comparison -- --save-baseline risc0
//!    ```
//!
//! 2. Run with Stub and compare:
//!    ```bash
//!    cargo bench --package zk --no-default-features --features stub \
//!      --bench backend_comparison -- --baseline risc0
//!    ```
//!
//! 3. View comparison in terminal or open HTML report:
//!    ```bash
//!    open target/criterion/backend_comparison/report/index.html
//!    ```
//!
//! ## Note on Arkworks
//!
//! Arkworks backend is currently excluded from this benchmark because the
//! GameTransitionCircuit uses Poseidon hash gadgets that don't have full
//! R1CS constraints implemented yet (they compute hashes natively for witness
//! generation but don't add proper circuit constraints). Once the Poseidon
//! constraint gadgets are fully implemented, arkworks can be added back.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use game_core::{
    ActorState, Action, ActionInput, ActionKind, CardinalDirection, CharacterAction, CoreStats,
    EntityId, EntitiesState, GameState, InventoryState, Position, TurnState, WorldState,
};

#[cfg(feature = "risc0")]
use zk::{OracleSnapshot, Risc0Prover};

#[cfg(feature = "stub")]
use zk::StubProver;

use zk::Prover;

/// Create a test game state with a given number of actors.
fn create_test_state(num_actors: usize) -> GameState {
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

    // Add player
    let player = ActorState::new(
        EntityId::PLAYER,
        Position::new(5, 5),
        default_stats.clone(),
        InventoryState::empty(),
    );
    let _ = entities.actors.push(player);

    // Add additional actors
    for i in 1..num_actors {
        let actor = ActorState::new(
            EntityId(i as u32),
            Position::new((5 + i as i32) % 50, (5 + i as i32) % 50),
            default_stats.clone(),
            InventoryState::empty(),
        );
        let _ = entities.actors.push(actor);
    }

    let mut active_actors = std::collections::BTreeSet::new();
    active_actors.insert(EntityId::PLAYER);

    let turn = TurnState {
        current_actor: EntityId::PLAYER,
        clock: 0,
        nonce: 0,
        active_actors,
    };

    GameState::with_seed(12345, turn, entities, WorldState::default())
}

// Helper to create minimal oracle snapshot for backends that need it
#[cfg(any(feature = "risc0", feature = "stub"))]
fn create_oracle_snapshot() -> zk::OracleSnapshot {
    use game_core::{GameConfig, MapDimensions};
    use zk::{ActorsSnapshot, ConfigSnapshot, ItemsSnapshot, MapSnapshot, OracleSnapshot, TablesSnapshot};

    OracleSnapshot::new(
        MapSnapshot {
            dimensions: MapDimensions { width: 50, height: 50 },
            tiles: vec![],
        },
        ItemsSnapshot::empty(),
        ActorsSnapshot::empty(),
        // TablesSnapshot requires proper initialization - use minimal valid data
        TablesSnapshot::new(
            game_core::ActionCosts {
                attack: 100,
                move_action: 100,
                wait: 0,
                interact: 100,
                use_item: 100,
                activation: 0,
            },
            game_core::CombatParams {
                hit_chance: game_core::HitChanceParams {
                    base: 85,
                    min: 5,
                    max: 95,
                },
                damage: game_core::DamageParams {
                    ac_divisor: 10,
                    crit_multiplier: 2,
                    minimum: 1,
                },
            },
            game_core::SpeedParams {
                cost_multiplier: 1000,
                min: 1,
                max: 1000,
            },
            std::collections::BTreeMap::new(),
        ),
        ConfigSnapshot::new(GameConfig::default()),
    )
}

#[cfg(feature = "risc0")]
fn create_prover() -> Risc0Prover {
    Risc0Prover::new(create_oracle_snapshot())
}

#[cfg(feature = "stub")]
fn create_prover() -> StubProver {
    StubProver::new(create_oracle_snapshot())
}

fn bench_proof_generation(c: &mut Criterion) {
    let backend_name = if cfg!(feature = "risc0") {
        "risc0"
    } else if cfg!(feature = "stub") {
        "stub"
    } else {
        "unknown"
    };

    let mut group = c.benchmark_group(format!("backend_comparison/{}", backend_name));

    // Test with different state sizes
    for num_actors in [1, 5, 10] {
        let before_state = create_test_state(num_actors);
        let mut after_state = before_state.clone();

        // Simulate a move action
        after_state.entities.actors[0].position = Position::new(6, 6);
        after_state.turn.nonce = 1;

        let action = Action::Character(CharacterAction {
            actor: EntityId::PLAYER,
            kind: ActionKind::Move,
            input: ActionInput::Direction(CardinalDirection::North),
        });

        let prover = create_prover();

        group.bench_function(format!("prove_move_{}_actors", num_actors), |b| {
            b.iter(|| {
                let proof = prover
                    .prove(
                        black_box(&before_state),
                        black_box(&action),
                        black_box(&after_state),
                    )
                    .expect("Proof generation failed");
                black_box(proof)
            });
        });
    }

    group.finish();
}

fn bench_proof_verification(c: &mut Criterion) {
    let backend_name = if cfg!(feature = "risc0") {
        "risc0"
    } else if cfg!(feature = "stub") {
        "stub"
    } else {
        "unknown"
    };

    let mut group = c.benchmark_group(format!("backend_comparison/{}/verify", backend_name));

    let before_state = create_test_state(5);
    let mut after_state = before_state.clone();
    after_state.entities.actors[0].position = Position::new(6, 6);
    after_state.turn.nonce = 1;

    let action = Action::Character(CharacterAction {
        actor: EntityId::PLAYER,
        kind: ActionKind::Move,
        input: ActionInput::Direction(CardinalDirection::North),
    });

    let prover = create_prover();
    let proof = prover
        .prove(&before_state, &action, &after_state)
        .expect("Proof generation failed");

    group.bench_function("verify_proof", |b| {
        b.iter(|| {
            let result = prover.verify(black_box(&proof)).expect("Verification failed");
            black_box(result)
        });
    });

    group.finish();
}

fn bench_proof_size(c: &mut Criterion) {
    let backend_name = if cfg!(feature = "risc0") {
        "risc0"
    } else if cfg!(feature = "stub") {
        "stub"
    } else {
        "unknown"
    };

    // Generate a proof and report its size
    let before_state = create_test_state(5);
    let mut after_state = before_state.clone();
    after_state.entities.actors[0].position = Position::new(6, 6);
    after_state.turn.nonce = 1;

    let action = Action::Character(CharacterAction {
        actor: EntityId::PLAYER,
        kind: ActionKind::Move,
        input: ActionInput::Direction(CardinalDirection::North),
    });

    let prover = create_prover();
    let proof = prover
        .prove(&before_state, &action, &after_state)
        .expect("Proof generation failed");

    println!(
        "\n📊 {} Proof Size: {} bytes\n",
        backend_name.to_uppercase(),
        proof.bytes.len()
    );

    // Dummy benchmark to show the size in reports
    c.bench_function(
        &format!("backend_comparison/{}/proof_size", backend_name),
        |b| {
            b.iter(|| black_box(proof.bytes.len()));
        },
    );
}

criterion_group!(
    benches,
    bench_proof_generation,
    bench_proof_verification,
    bench_proof_size
);
criterion_main!(benches);
