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

use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};

use game_core::{Action, GameState, StateDelta, Tick};
use zk::{ProofData, ProofError};

use crate::events::{Event, EventBus, GameStateEvent, ProofEvent, Topic};
use crate::oracle::OracleManager;
use crate::workers::ProofMetrics;

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
/// - **Phase 2 (Current)**: RISC0 zkVM with oracle snapshots
/// - **Phase 4 (Future)**: Incremental Merkle tree updates
pub struct ProverWorker {
    /// Current game state (synchronized with simulation worker via events)
    current_state: GameState,

    /// Oracle manager for creating oracle snapshots
    oracle_manager: OracleManager,

    /// Receives game state events (especially ActionExecuted)
    event_rx: broadcast::Receiver<Event>,

    /// Event bus for publishing proof events
    event_bus: EventBus,

    /// Proof generation metrics (shared with RuntimeHandle for querying)
    /// Uses atomics for lock-free access
    metrics: Arc<ProofMetrics>,

    /// Optional directory path for saving proofs
    save_proofs_dir: Option<std::path::PathBuf>,
}

impl ProverWorker {
    /// Creates a new prover worker.
    ///
    /// # Arguments
    ///
    /// * `initial_state` - Initial game state (synchronized with simulation worker)
    /// * `event_bus` - Event bus for subscribing to GameState events and publishing Proof events
    /// * `oracle_manager` - Oracle manager for creating snapshots
    /// * `save_proofs_dir` - Optional directory to save proof files
    pub fn new(
        initial_state: GameState,
        event_bus: EventBus,
        oracle_manager: OracleManager,
        save_proofs_dir: Option<std::path::PathBuf>,
    ) -> Self {
        // Subscribe to GameState topic only
        let event_rx = event_bus.subscribe(Topic::GameState);

        Self {
            current_state: initial_state,
            oracle_manager,
            event_rx,
            event_bus,
            metrics: Arc::new(ProofMetrics::new()),
            save_proofs_dir,
        }
    }

    /// Returns a clone of the metrics Arc for external querying.
    pub fn metrics(&self) -> Arc<ProofMetrics> {
        Arc::clone(&self.metrics)
    }

    /// Creates an oracle snapshot from the oracle manager.
    ///
    /// This is a helper to avoid code duplication between proof generation
    /// and verification.
    fn create_oracle_snapshot(oracle_manager: &OracleManager) -> zk::OracleSnapshot {
        use zk::{
            ConfigSnapshot, ItemsSnapshot, MapSnapshot, NpcsSnapshot, OracleSnapshot,
            TablesSnapshot,
        };

        let map_snapshot = MapSnapshot::from_oracle(oracle_manager.map.as_ref());
        let items_snapshot = ItemsSnapshot::empty(); // TODO: Populate with actual items
        let npcs_snapshot = NpcsSnapshot::empty(); // TODO: Populate with actual NPCs
        let tables_snapshot = TablesSnapshot::from_oracle(oracle_manager.tables.as_ref());
        let config_snapshot = ConfigSnapshot::from_oracle(oracle_manager.config.as_ref());

        OracleSnapshot::new(
            map_snapshot,
            items_snapshot,
            npcs_snapshot,
            tables_snapshot,
            config_snapshot,
        )
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
    async fn handle_event(&mut self, event: Event) {
        if let Event::GameState(GameStateEvent::ActionExecuted {
            action,
            delta,
            clock,
            before_state,
            after_state,
        }) = event
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

        // Update queue depth metric (incremented when starting) - lock-free
        let new_depth = self.metrics.queue_depth() + 1;
        self.metrics.set_queue_depth(new_depth);

        // Emit proof started event to Proof topic
        self.event_bus
            .publish(Event::Proof(ProofEvent::ProofStarted {
                action: action.clone(),
                clock,
            }));

        let event_received_time = std::time::Instant::now();

        // Phase 2: Full rebuild approach
        // TODO: This will be replaced with incremental updates in Phase 4
        match self
            .generate_proof_full_rebuild(
                &action,
                &delta,
                &before_state,
                &after_state,
                event_received_time,
            )
            .await
        {
            Ok((proof_data, wait_time, proving_time)) => {
                let generation_time_ms = proving_time.as_millis() as u64;
                let wait_time_ms = wait_time.as_millis() as u64;

                info!(
                    "Proof generated for action at tick {} (wait: {}ms, proving: {}ms)",
                    clock, wait_time_ms, generation_time_ms
                );

                // NOTE: Proof is already verified by RISC0 during prove().
                // No need to verify again in the same process immediately after generation.
                // The verify() method will be used when loading proofs from files or external sources.

                // Update metrics - lock-free atomic operations
                self.metrics.record_wait(wait_time);
                self.metrics.record_success(proving_time);
                let new_depth = self.metrics.queue_depth().saturating_sub(1);
                self.metrics.set_queue_depth(new_depth);

                // Save proof to file if configured
                if let Some(ref dir) = self.save_proofs_dir {
                    self.save_proof_to_file(dir, &action, clock, &proof_data)
                        .await;
                }

                // Publish proof generated event to Proof topic
                self.event_bus
                    .publish(Event::Proof(ProofEvent::ProofGenerated {
                        action,
                        clock,
                        proof_data,
                        generation_time_ms,
                    }));
            }
            Err(error) => {
                // Log error with appropriate severity
                match &error {
                    ProofError::StateInconsistency(_) => {
                        // This is CRITICAL - indicates determinism bug
                        error!(
                            target: "runtime::prover",
                            "🚨 CRITICAL: State inconsistency detected! zkVM and simulation computed different results. {}",
                            error
                        );
                    }
                    _ => {
                        error!("Proof generation failed: {}", error);
                    }
                }

                // Update failure metrics - lock-free atomic operations
                self.metrics.record_failure();
                let new_depth = self.metrics.queue_depth().saturating_sub(1);
                self.metrics.set_queue_depth(new_depth);

                // Publish proof failed event to Proof topic
                self.event_bus
                    .publish(Event::Proof(ProofEvent::ProofFailed {
                        action,
                        clock,
                        error: error.to_string(),
                    }));
            }
        }

        // Update our state reference for next iteration
        self.current_state = after_state;
    }

    /// Saves proof to file if directory is configured.
    async fn save_proof_to_file(
        &self,
        dir: &std::path::Path,
        action: &Action,
        clock: Tick,
        proof_data: &ProofData,
    ) {
        use tokio::fs;

        // Create directory if it doesn't exist
        if let Err(e) = fs::create_dir_all(dir).await {
            warn!("Failed to create proof directory {:?}: {}", dir, e);
            return;
        }

        // Generate filename: proof_{actor}_{kind}_tick_{clock}.bin
        let kind_str = action.kind.as_snake_case();
        let filename = format!("proof_{}_{}_{}.bin", action.actor, kind_str, clock);
        let filepath = dir.join(filename);

        // Save proof bytes
        match fs::write(&filepath, &proof_data.bytes).await {
            Ok(_) => {
                info!(
                    "💾 Proof saved: {} ({} bytes, backend: {:?})",
                    filepath.display(),
                    proof_data.bytes.len(),
                    proof_data.backend
                );
            }
            Err(e) => {
                warn!("Failed to save proof to {:?}: {}", filepath, e);
            }
        }
    }

    /// Generates a zero-knowledge proof for an action execution.
    ///
    /// Creates an oracle snapshot and invokes the zkVM prover to generate
    /// a proof that executing the action on before_state produces after_state.
    ///
    /// Returns a tuple of (ProofData, wait_time, proving_time) where:
    /// - wait_time: Duration between event received and blocking task started
    /// - proving_time: Duration of actual proof generation
    ///
    /// # Performance
    ///
    /// - Production: 30-60 seconds (with GPU/Metal acceleration)
    /// - Development (RISC0_DEV_MODE=1): <100ms (mock proofs)
    async fn generate_proof_full_rebuild(
        &self,
        action: &Action,
        delta: &StateDelta,
        before_state: &GameState,
        after_state: &GameState,
        event_received_time: std::time::Instant,
    ) -> Result<(ProofData, std::time::Duration, std::time::Duration), ProofError> {
        // Use the configured prover from zk crate (determined by zk's feature flags)
        use zk::{Prover, ZkProver};

        // Clone oracle manager to send to blocking task
        let oracle_manager = self.oracle_manager.clone();
        let before_state = before_state.clone();
        let action = action.clone();
        let after_state = after_state.clone();
        let delta = delta.clone();

        // Generate proof (may take seconds for real proofs)
        // Use tokio::spawn_blocking to avoid blocking the async runtime
        let proof = tokio::task::spawn_blocking(move || {
            // Measure wait time (from event received to blocking task started)
            let blocking_task_started = std::time::Instant::now();
            let wait_time = blocking_task_started.duration_since(event_received_time);

            // Measure proving time
            let proving_start = std::time::Instant::now();
            let oracle_snapshot = Self::create_oracle_snapshot(&oracle_manager);
            let prover = ZkProver::new(oracle_snapshot);
            let result = prover.prove(&before_state, &action, &after_state, &delta);
            let proving_time = proving_start.elapsed();

            result.map(|proof_data| (proof_data, wait_time, proving_time))
        })
        .await
        .map_err(|e| ProofError::ZkvmError(format!("Proof task failed: {}", e)))??;

        Ok(proof)
    }
}
