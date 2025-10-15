//! ZK proof generation worker.
//!
//! This worker subscribes to game events and generates zero-knowledge proofs
//! for executed actions. It maintains its own copy of the game state and
//! incremental Merkle trees (in Phase 4) to efficiently generate witnesses.
//!
//! Design principles:
//! - Completely isolated from game logic (only processes deltas)
//! - Runs asynchronously without blocking game execution
//! - Maintains its own copy of state + Merkle trees
//! - Emits proof events for clients/submitters to consume

use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};

use game_core::{Action, GameState, StateDelta, Tick};
use zk::{ProofBackend, ProofData, ProofError};

use crate::api::GameEvent;

/// Background worker for ZK proof generation.
///
/// Subscribes to ActionExecuted events, generates zero-knowledge proofs,
/// and broadcasts ProofGenerated/ProofFailed events.
///
/// # Architecture
///
/// ```text
/// SimulationWorker                    ProverWorker
///       │                                  │
///       │ ActionExecuted event             │
///       │ (action, delta, before, after)   │
///       ├─────────────────────────────────▶│
///       │                                  │ Update state
///       │                                  │ Generate proof (async)
///       │                                  │
///       │                        ProofGenerated event
///       │◀─────────────────────────────────┤
///       │                                  │
/// ```
///
/// # Phases
///
/// - **Phase 2 (Current)**: Full Merkle tree rebuild per action
/// - **Phase 4 (Future)**: Incremental Merkle tree updates
pub struct ProverWorker {
    /// Current game state (synchronized with simulation worker via events)
    current_state: GameState,

    /// Incremental Merkle tree for entities (Phase 4)
    /// Currently None - will be implemented when needed
    #[allow(dead_code)]
    entity_tree: Option<()>, // Placeholder for IncrementalMerkleTree

    /// Incremental Merkle tree for world tiles (Phase 4)
    /// Currently None - will be implemented when needed
    #[allow(dead_code)]
    world_tree: Option<()>, // Placeholder for IncrementalMerkleTree

    /// Receives game events (especially ActionExecuted)
    event_rx: broadcast::Receiver<GameEvent>,

    /// Broadcasts proof generation events (same channel as game events)
    event_tx: broadcast::Sender<GameEvent>,
}

impl ProverWorker {
    /// Creates a new prover worker.
    ///
    /// # Arguments
    ///
    /// * `initial_state` - Initial game state (synchronized with simulation worker)
    /// * `event_rx` - Receiver for game events (subscribed to ActionExecuted)
    /// * `event_tx` - Sender for proof events (ProofGenerated, ProofFailed)
    pub fn new(
        initial_state: GameState,
        event_rx: broadcast::Receiver<GameEvent>,
        event_tx: broadcast::Sender<GameEvent>,
    ) -> Self {
        Self {
            current_state: initial_state,
            entity_tree: None,
            world_tree: None,
            event_rx,
            event_tx,
        }
    }

    /// Main worker loop.
    ///
    /// Processes game events until the channel is closed.
    /// Handles lagged events gracefully by skipping them.
    pub async fn run(mut self) {
        info!("ProverWorker started");

        loop {
            match self.event_rx.recv().await {
                Ok(event) => {
                    self.handle_event(event).await;
                }
                Err(broadcast::error::RecvError::Lagged(skipped)) => {
                    warn!(
                        "ProverWorker lagged, skipped {} events - proofs may be missing",
                        skipped
                    );
                    // Continue processing (we might miss some proofs but that's ok)
                }
                Err(broadcast::error::RecvError::Closed) => {
                    info!("ProverWorker shutting down (event channel closed)");
                    break;
                }
            }
        }

        info!("ProverWorker stopped");
    }

    /// Handles a single game event.
    ///
    /// Currently only processes ActionExecuted events.
    /// Other event types are ignored.
    async fn handle_event(&mut self, event: GameEvent) {
        if let GameEvent::ActionExecuted {
            action,
            delta,
            clock,
            before_state,
            after_state,
        } = event
        {
            self.handle_action_executed(action, *delta, clock, *before_state, *after_state)
                .await;
        }
    }

    /// Processes an executed action and generates a proof.
    ///
    /// # Workflow
    ///
    /// 1. Announce proof generation started
    /// 2. Generate proof (Phase 2: full rebuild, Phase 4: incremental)
    /// 3. Broadcast ProofGenerated or ProofFailed event
    /// 4. Update internal state reference
    ///
    /// # Performance
    ///
    /// Phase 2: ~10ms (full tree rebuild) + seconds (proof generation)
    /// Phase 4: ~0.5ms (incremental update) + seconds (proof generation)
    async fn handle_action_executed(
        &mut self,
        action: Action,
        delta: StateDelta,
        clock: Tick,
        before_state: GameState,
        after_state: GameState,
    ) {
        debug!("ProverWorker processing action at tick {}", clock);

        // Emit proof started event
        let _ = self.event_tx.send(GameEvent::ProofStarted {
            action: action.clone(),
            clock,
        });

        let start = std::time::Instant::now();

        // Phase 2: Full rebuild approach
        // TODO: This will be replaced with incremental updates in Phase 4
        match self
            .generate_proof_full_rebuild(&action, &delta, &before_state, &after_state)
            .await
        {
            Ok(proof_data) => {
                let generation_time_ms = start.elapsed().as_millis() as u64;
                info!(
                    "Proof generated for action at tick {} in {}ms",
                    clock, generation_time_ms
                );

                let _ = self.event_tx.send(GameEvent::ProofGenerated {
                    action,
                    clock,
                    proof_data,
                    generation_time_ms,
                });
            }
            Err(error) => {
                error!("Proof generation failed: {}", error);

                let _ = self.event_tx.send(GameEvent::ProofFailed {
                    action,
                    clock,
                    error: error.to_string(),
                });
            }
        }

        // Update our state reference for next iteration
        self.current_state = after_state;
    }

    /// Generates a proof using full Merkle tree rebuild (Phase 2).
    ///
    /// This is a placeholder implementation that will be replaced with
    /// actual ZK proof generation once the `zk` crate is implemented.
    ///
    /// # Phase 2 Algorithm
    ///
    /// 1. Build full Merkle trees from before_state
    /// 2. Build full Merkle trees from after_state
    /// 3. Generate witnesses using delta as guide (only changed entities)
    /// 4. Generate ZK proof using witnesses
    ///
    /// # Future Work
    ///
    /// This will call into `zk::StateTransition::from_delta()` once implemented.
    async fn generate_proof_full_rebuild(
        &self,
        _action: &Action,
        _delta: &StateDelta,
        _before_state: &GameState,
        _after_state: &GameState,
    ) -> Result<ProofData, ProofError> {
        // Phase 2 stub implementation
        // TODO: Replace with actual proof generation:
        //
        // #[cfg(feature = "zkvm")]
        // use zk::zkvm::DefaultProver;
        //
        // let prover = DefaultProver::new();
        // let proof = prover.prove(before_state, action, after_state, delta)?;
        //
        // Or for custom circuit:
        // #[cfg(feature = "custom-circuit")]
        // use zk::circuit::StateTransition;
        //
        // let transition = StateTransition::from_delta(delta, before_state, after_state)?;
        // let proof = transition.prove()?;

        // For now, return a placeholder proof
        Ok(ProofData {
            bytes: vec![0xDE, 0xAD, 0xBE, 0xEF], // Dummy proof
            backend: ProofBackend::None,
        })
    }
}

// ProofError is now re-exported from zk crate
