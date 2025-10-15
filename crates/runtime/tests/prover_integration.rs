//! Integration test for ProverWorker functionality
//!
//! This test verifies that ProverWorker correctly subscribes to ActionExecuted events
//! and generates ProofStarted/ProofGenerated events.

use game_core::{Action, ActionKind, EntityId};
use runtime::{GameEvent, Runtime, RuntimeConfig};
use tokio::time::{Duration, timeout};

#[tokio::test]
async fn test_prover_worker_generates_proof_events() {
    // Create runtime with proving enabled
    let config = RuntimeConfig {
        enable_proving: true,
        ..Default::default()
    };

    let runtime = create_test_runtime(config).await;
    let mut event_rx = runtime.subscribe_events();

    // Prepare turn
    let handle = runtime.handle();
    let (entity, _state) = handle
        .prepare_next_turn()
        .await
        .expect("Failed to prepare turn");

    assert_eq!(entity, EntityId::PLAYER);

    // Execute a simple wait action
    let wait_action = Action::new(EntityId::PLAYER, ActionKind::Wait);

    handle
        .execute_action(wait_action.clone())
        .await
        .expect("Failed to execute action");

    // Collect events with timeout
    let mut saw_action_executed = false;
    let mut saw_proof_started = false;
    let mut saw_proof_generated = false;

    let event_collection = timeout(Duration::from_secs(2), async {
        loop {
            match event_rx.recv().await {
                Ok(GameEvent::TurnCompleted { .. }) => {}
                Ok(GameEvent::ActionExecuted { action, .. }) => {
                    assert_eq!(action.actor, EntityId::PLAYER);
                    saw_action_executed = true;
                }
                Ok(GameEvent::ProofStarted { action, .. }) => {
                    assert_eq!(action.actor, EntityId::PLAYER);
                    saw_proof_started = true;
                }
                Ok(GameEvent::ProofGenerated {
                    action,
                    generation_time_ms,
                    ..
                }) => {
                    assert_eq!(action.actor, EntityId::PLAYER);
                    println!("Proof generated in {}ms", generation_time_ms);
                    saw_proof_generated = true;
                    break; // Got what we need
                }
                Ok(GameEvent::ProofFailed { error, .. }) => {
                    panic!("Proof generation failed: {}", error);
                }
                Ok(GameEvent::ActionFailed { error, .. }) => {
                    panic!("Action failed: {}", error);
                }
                Err(e) => {
                    panic!("Failed to receive event: {}", e);
                }
            }
        }
    })
    .await;

    assert!(
        event_collection.is_ok(),
        "Timed out waiting for proof events"
    );
    assert!(saw_action_executed, "Never received ActionExecuted event");
    assert!(saw_proof_started, "Never received ProofStarted event");
    assert!(saw_proof_generated, "Never received ProofGenerated event");

    // Cleanup
    runtime
        .shutdown()
        .await
        .expect("Failed to shutdown runtime");
}

#[tokio::test]
async fn test_prover_worker_disabled_by_default() {
    // Build without enable_proving (uses default config)
    let runtime = create_test_runtime(RuntimeConfig::default()).await;
    let mut event_rx = runtime.subscribe_events();

    // Prepare and execute action
    let handle = runtime.handle();
    handle
        .prepare_next_turn()
        .await
        .expect("Failed to prepare turn");

    let wait_action = Action::new(EntityId::PLAYER, ActionKind::Wait);

    handle
        .execute_action(wait_action)
        .await
        .expect("Failed to execute action");

    // Check that we DON'T receive proof events
    let mut saw_proof_event = false;
    let mut saw_action_executed = false;

    let event_collection = timeout(Duration::from_millis(500), async {
        loop {
            match event_rx.recv().await {
                Ok(GameEvent::ActionExecuted { .. }) => {
                    saw_action_executed = true;
                    // Wait a bit more to ensure no proof events come
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    break;
                }
                Ok(GameEvent::ProofStarted { .. })
                | Ok(GameEvent::ProofGenerated { .. })
                | Ok(GameEvent::ProofFailed { .. }) => {
                    saw_proof_event = true;
                    break;
                }
                Ok(_) => {} // Other events are fine
                Err(_) => break,
            }
        }
    })
    .await;

    assert!(event_collection.is_ok(), "Timed out unexpectedly");
    assert!(saw_action_executed, "Never received ActionExecuted event");
    assert!(
        !saw_proof_event,
        "Received proof event when proving was disabled"
    );

    runtime
        .shutdown()
        .await
        .expect("Failed to shutdown runtime");
}

// Helper function to create test runtime
async fn create_test_runtime(config: RuntimeConfig) -> Runtime {
    use client_core::config::MapSize;
    use client_core::world::{OracleFactory, TestOracleFactory};

    let factory = TestOracleFactory::new(MapSize {
        width: 20,
        height: 20,
    });
    let oracles = factory.build().manager();

    Runtime::builder()
        .config(config)
        .oracles(oracles)
        .build()
        .await
        .expect("Failed to build runtime")
}
