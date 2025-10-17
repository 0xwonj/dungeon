//! ZK proof generation worker.
//!
//! This worker reads from the action log and generates zero-knowledge proofs
//! for executed actions. It maintains a cursor into the action log and
//! processes entries sequentially.
//!
//! Design principles:
//! - Reads from action log rather than subscribing to events
//! - Runs asynchronously without blocking game execution
//! - Emits proof events for clients/submitters to consume
//! - Can resume from checkpoint to avoid re-proving

use std::path::PathBuf;
use std::sync::Arc;
use tokio::time::{self, Duration};
use tracing::{debug, error, info, warn};

use game_core::{Action, GameState};
use zk::{ProofData, ProofError, Prover, ZkProver};

use crate::events::{Event, EventBus, ProofEvent};
use crate::oracle::OracleManager;
use crate::repository::{ActionLogEntry, FileActionLog};
use crate::workers::ProofMetrics;

/// Background worker for ZK proof generation.
///
/// Reads ActionLogEntry records from the action log, generates zero-knowledge proofs,
/// and broadcasts ProofGenerated/ProofFailed events.
pub struct ProverWorker {
    /// ZK prover instance
    prover: ZkProver,

    /// Action log repository
    action_log: FileActionLog,

    /// Current byte offset in the action log
    current_offset: u64,

    /// Event bus for publishing proof events
    event_bus: EventBus,

    /// Proof generation metrics (shared with RuntimeHandle for querying)
    /// Uses atomics for lock-free access
    metrics: Arc<ProofMetrics>,

    /// Optional directory path for saving proofs
    save_proofs_dir: Option<PathBuf>,

    /// Polling interval for checking new actions
    poll_interval: Duration,
}

impl ProverWorker {
    /// Creates a new prover worker.
    ///
    /// # Arguments
    ///
    /// * `action_log` - Action log repository to read from
    /// * `event_bus` - Event bus for publishing Proof events
    /// * `oracle_manager` - Oracle manager for creating snapshots
    /// * `save_proofs_dir` - Optional directory to save proof files
    /// * `start_offset` - Byte offset to start reading from (0 for beginning, or checkpoint offset)
    pub fn new(
        action_log: FileActionLog,
        event_bus: EventBus,
        oracle_manager: OracleManager,
        save_proofs_dir: Option<PathBuf>,
        start_offset: u64,
    ) -> Self {
        // Create prover
        let oracle_snapshot = Self::create_oracle_snapshot(&oracle_manager);
        let prover = ZkProver::new(oracle_snapshot);

        Self {
            prover,
            action_log,
            current_offset: start_offset,
            event_bus,
            metrics: Arc::new(ProofMetrics::new()),
            save_proofs_dir,
            poll_interval: Duration::from_millis(100),
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
    /// Polls the action log for new entries and generates proofs.
    pub async fn run(mut self) {
        info!(
            "ProverWorker started (offset: {}, session: {})",
            self.current_offset,
            self.action_log.session_id()
        );

        loop {
            // Try to read next entry from action log
            match self.action_log.read_at_offset(self.current_offset) {
                Ok(Some(entry)) => {
                    // Process the entry
                    self.handle_action_entry(entry).await;

                    // Advance offset (we don't know exact size, so we'll retry from same offset if needed)
                    // In practice, the action log implementation should track offsets properly
                    match self.action_log.size() {
                        Ok(size) if size > self.current_offset => {
                            // There might be more entries, continue immediately
                            continue;
                        }
                        _ => {
                            // No more entries yet, sleep and poll again
                            time::sleep(self.poll_interval).await;
                        }
                    }
                }
                Ok(None) => {
                    // No more entries available, sleep and poll again
                    time::sleep(self.poll_interval).await;
                }
                Err(e) => {
                    error!(
                        "Failed to read action log at offset {}: {}",
                        self.current_offset, e
                    );
                    time::sleep(self.poll_interval).await;
                }
            }
        }
    }

    /// Processes a single action log entry and generates a proof.
    ///
    /// # Workflow
    ///
    /// 1. Announce proof generation started
    /// 2. Generate proof
    /// 3. Broadcast ProofGenerated or ProofFailed event
    /// 4. Update offset
    async fn handle_action_entry(&mut self, entry: ActionLogEntry) {
        let ActionLogEntry {
            nonce,
            clock,
            action,
            before_state,
            after_state,
            delta: _,
        } = entry;

        debug!(
            "ProverWorker processing action nonce={} tick={}",
            nonce, clock
        );

        // Update queue depth metric (incremented when starting) - lock-free
        let new_depth = self.metrics.queue_depth() + 1;
        self.metrics.set_queue_depth(new_depth);

        // Emit proof started event to Proof topic
        self.event_bus
            .publish(Event::Proof(ProofEvent::ProofStarted {
                action: action.clone(),
                clock,
            }));

        // Generate proof
        match self
            .generate_proof(&action, &*before_state, &*after_state)
            .await
        {
            Ok((proof_data, proving_time)) => {
                let generation_time_ms = proving_time.as_millis() as u64;

                info!(
                    "Proof generated for nonce={} tick={} (proving: {}ms)",
                    nonce, clock, generation_time_ms
                );

                // Update metrics - lock-free atomic operations
                self.metrics.record_success(proving_time);
                let new_depth = self.metrics.queue_depth().saturating_sub(1);
                self.metrics.set_queue_depth(new_depth);

                // Save proof to file if configured
                if let Some(ref dir) = self.save_proofs_dir {
                    self.save_proof_to_file(dir, &action, nonce, &proof_data)
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
                            "🚨 CRITICAL: State inconsistency detected at nonce={}! zkVM and simulation computed different results. {}",
                            nonce, error
                        );
                    }
                    _ => {
                        error!("Proof generation failed at nonce={}: {}", nonce, error);
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

        // Update offset to next entry
        // Note: In the real implementation, we need to calculate the exact byte size
        // For now, we increment by a placeholder (the action log should track this)
        self.current_offset = self.action_log.size().unwrap_or(self.current_offset);
    }

    /// Saves proof to file if directory is configured.
    async fn save_proof_to_file(
        &self,
        dir: &std::path::Path,
        action: &Action,
        nonce: u64,
        proof_data: &ProofData,
    ) {
        use tokio::fs;

        // Create directory if it doesn't exist
        if let Err(e) = fs::create_dir_all(dir).await {
            warn!("Failed to create proof directory {:?}: {}", dir, e);
            return;
        }

        // Generate filename: proof_#{actor}_{kind}_{nonce}.bin
        let kind_str = action.kind.as_snake_case();
        let filename = format!("proof_#{}_{}_{}.bin", action.actor, kind_str, nonce);
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
    /// Returns a tuple of (ProofData, proving_time) where:
    /// - proving_time: Duration of actual proof generation
    async fn generate_proof(
        &self,
        action: &Action,
        before_state: &GameState,
        after_state: &GameState,
    ) -> Result<(ProofData, std::time::Duration), ProofError> {
        // Clone prover and states to send to blocking task
        let prover = self.prover.clone();
        let before_state = before_state.clone();
        let action = action.clone();
        let after_state = after_state.clone();

        // Generate proof (may take seconds for real proofs)
        // Use tokio::spawn_blocking to avoid blocking the async runtime
        let proof = tokio::task::spawn_blocking(move || {
            // Measure proving time
            let proving_start = std::time::Instant::now();
            let result = prover.prove(&before_state, &action, &after_state);
            let proving_time = proving_start.elapsed();

            result.map(|proof_data| (proof_data, proving_time))
        })
        .await
        .map_err(|e| ProofError::ZkvmError(format!("Proof task failed: {}", e)))??;

        Ok(proof)
    }
}
