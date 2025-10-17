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
use crate::repository::{
    ActionLogEntry, ActionRepository, ProofEntry, ProofIndex, ProofIndexRepository,
};
use crate::workers::ProofMetrics;

/// Action log reader that tracks current read position.
struct ActionLogReader {
    /// Action log repository
    log: Box<dyn ActionRepository>,

    /// Current byte offset in the action log
    current_offset: u64,

    /// Polling interval for checking new actions
    poll_interval: Duration,
}

impl ActionLogReader {
    fn new(log: Box<dyn ActionRepository>, start_offset: u64) -> Self {
        Self {
            log,
            current_offset: start_offset,
            poll_interval: Duration::from_millis(100),
        }
    }

    fn session_id(&self) -> &str {
        self.log.session_id()
    }

    fn read_next(
        &mut self,
    ) -> Result<Option<(ActionLogEntry, u64)>, crate::api::errors::RuntimeError> {
        self.log.read_at_offset(self.current_offset)
    }

    fn size(&self) -> Result<u64, crate::api::errors::RuntimeError> {
        self.log.size()
    }

    fn advance_to(&mut self, offset: u64) {
        self.current_offset = offset;
    }

    fn current_offset(&self) -> u64 {
        self.current_offset
    }

    fn poll_interval(&self) -> Duration {
        self.poll_interval
    }
}

/// Proof persistence layer (disk I/O).
struct ProofStorage {
    /// Proof index repository for tracking which proofs have been generated
    index_repo: Box<dyn ProofIndexRepository>,

    /// Optional directory path for saving proof files
    save_dir: Option<PathBuf>,
}

impl ProofStorage {
    fn new(index_repo: Box<dyn ProofIndexRepository>, save_dir: Option<PathBuf>) -> Self {
        Self {
            index_repo,
            save_dir,
        }
    }

    fn save_index(&self, index: &ProofIndex) -> Result<(), crate::api::errors::RuntimeError> {
        self.index_repo.save(index)
    }

    fn save_dir(&self) -> Option<&PathBuf> {
        self.save_dir.as_ref()
    }
}

/// Background worker for ZK proof generation.
///
/// Reads ActionLogEntry records from the action log, generates zero-knowledge proofs,
/// and broadcasts ProofGenerated/ProofFailed events.
pub struct ProverWorker {
    /// ZK prover instance
    prover: ZkProver,

    /// Action log reader
    reader: ActionLogReader,

    /// Event bus for publishing proof events
    event_bus: EventBus,

    /// Proof generation metrics (shared with RuntimeHandle for querying)
    /// Uses atomics for lock-free access
    metrics: Arc<ProofMetrics>,

    /// In-memory proof index (worker state)
    proof_index: ProofIndex,

    /// Proof persistence layer (disk I/O)
    storage: ProofStorage,
}

impl ProverWorker {
    /// Create a new builder for ProverWorker.
    pub fn builder() -> ProverWorkerBuilder {
        ProverWorkerBuilder::new()
    }

    /// Creates a new prover worker (used by builder).
    ///
    /// # Arguments
    ///
    /// * `action_log` - Action log repository to read from
    /// * `event_bus` - Event bus for publishing Proof events
    /// * `oracle_manager` - Oracle manager for creating snapshots
    /// * `save_proofs_dir` - Optional directory to save proof files
    /// * `proof_index_repo` - Proof index repository for tracking generated proofs
    /// * `session_id` - Session ID for this runtime instance
    /// * `start_offset` - Byte offset to start reading from (0 for beginning, or checkpoint offset)
    fn new(
        action_log: Box<dyn ActionRepository>,
        event_bus: EventBus,
        oracle_manager: OracleManager,
        save_proofs_dir: Option<PathBuf>,
        proof_index_repo: Box<dyn ProofIndexRepository>,
        session_id: String,
        start_offset: u64,
    ) -> Result<Self, String> {
        // Create prover
        let oracle_snapshot = Self::create_oracle_snapshot(&oracle_manager);
        let prover = ZkProver::new(oracle_snapshot);

        // Load or create proof index
        let proof_index = match proof_index_repo.load(&session_id) {
            Ok(Some(index)) => {
                info!(
                    "Loaded existing proof index: session={}, proven_up_to={}",
                    session_id, index.proven_up_to_nonce
                );
                index
            }
            Ok(None) => {
                info!("Creating new proof index: session={}", session_id);
                ProofIndex::new(session_id.clone())
            }
            Err(e) => {
                warn!("Failed to load proof index, creating new: {}", e);
                ProofIndex::new(session_id.clone())
            }
        };

        Ok(Self {
            prover,
            reader: ActionLogReader::new(action_log, start_offset),
            event_bus,
            metrics: Arc::new(ProofMetrics::new()),
            proof_index,
            storage: ProofStorage::new(proof_index_repo, save_proofs_dir),
        })
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
            self.reader.current_offset(),
            self.reader.session_id()
        );

        loop {
            // Try to read next entry from action log
            match self.reader.read_next() {
                Ok(Some((entry, next_offset))) => {
                    // Process the entry
                    self.handle_action_entry(entry).await;

                    // Update offset to point to the next entry
                    self.reader.advance_to(next_offset);

                    // Check if there might be more entries immediately available
                    match self.reader.size() {
                        Ok(size) if size > self.reader.current_offset() => {
                            // There might be more entries, continue immediately
                            continue;
                        }
                        _ => {
                            // No more entries yet, sleep and poll again
                            time::sleep(self.reader.poll_interval()).await;
                        }
                    }
                }
                Ok(None) => {
                    // No more entries available, sleep and poll again
                    time::sleep(self.reader.poll_interval()).await;
                }
                Err(e) => {
                    error!(
                        "Failed to read action log at offset {}: {}",
                        self.reader.current_offset(),
                        e
                    );
                    time::sleep(self.reader.poll_interval()).await;
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
            .generate_proof(&action, &before_state, &after_state)
            .await
        {
            Ok((proof_data, proving_time)) => {
                let generation_time_ms = proving_time.as_millis() as u64;

                // Compute state hashes for logging
                use crate::utils::hash::hash_game_state;
                let before_hash = hash_game_state(&before_state);
                let after_hash = hash_game_state(&after_state);

                info!(
                    "Proof generated for nonce={} tick={} (proving: {}ms) | before={} after={}",
                    nonce,
                    clock,
                    generation_time_ms,
                    &before_hash[..8],
                    &after_hash[..8]
                );

                // Update metrics - lock-free atomic operations
                self.metrics.record_success(proving_time);
                let new_depth = self.metrics.queue_depth().saturating_sub(1);
                self.metrics.set_queue_depth(new_depth);

                // Create proof entry
                let mut proof_entry = ProofEntry::new(nonce, generation_time_ms);

                // Save proof to file if configured
                if let Some(dir) = self.storage.save_dir()
                    && let Some((filename, size)) = self
                        .save_proof_to_file(dir, &action, nonce, &proof_data)
                        .await
                {
                    proof_entry = proof_entry.with_file(filename, size);
                }

                // Update proof index
                self.proof_index.add_proof(proof_entry);

                // Save proof index to disk (every proof for now, could batch later)
                if let Err(e) = self.storage.save_index(&self.proof_index) {
                    error!("Failed to save proof index: {}", e);
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

        // Offset is already updated in the main loop after read_at_offset returns next_offset
    }

    /// Saves proof to file if directory is configured.
    ///
    /// Returns Some((filename, size_bytes)) on success, None on failure.
    async fn save_proof_to_file(
        &self,
        dir: &std::path::Path,
        action: &Action,
        nonce: u64,
        proof_data: &ProofData,
    ) -> Option<(String, u64)> {
        use tokio::fs;

        // Create directory if it doesn't exist
        if let Err(e) = fs::create_dir_all(dir).await {
            warn!("Failed to create proof directory {:?}: {}", dir, e);
            return None;
        }

        // Generate filename: proof_{nonce}_{actor}_{kind}.bin
        let kind_str = action.as_snake_case();
        let filename = format!("proof_{}_{}_{}.bin", nonce, action.actor(), kind_str);
        let filepath = dir.join(&filename);

        let size_bytes = proof_data.bytes.len() as u64;

        // Save proof bytes
        match fs::write(&filepath, &proof_data.bytes).await {
            Ok(_) => {
                info!(
                    "💾 Proof saved: {} ({} bytes, backend: {:?})",
                    filepath.display(),
                    size_bytes,
                    proof_data.backend
                );
                Some((filename, size_bytes))
            }
            Err(e) => {
                warn!("Failed to save proof to {:?}: {}", filepath, e);
                None
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

/// Builder for [`ProverWorker`]
pub struct ProverWorkerBuilder {
    session_id: Option<String>,
    persistence_dir: Option<PathBuf>,
    event_bus: Option<EventBus>,
    oracles: Option<OracleManager>,
    save_proofs_dir: Option<PathBuf>,
    start_offset: u64,
}

impl ProverWorkerBuilder {
    fn new() -> Self {
        Self {
            session_id: None,
            persistence_dir: None,
            event_bus: None,
            oracles: None,
            save_proofs_dir: None,
            start_offset: 0,
        }
    }

    /// Set the session ID (required).
    pub fn session_id(mut self, id: impl Into<String>) -> Self {
        self.session_id = Some(id.into());
        self
    }

    /// Set the persistence base directory (required).
    ///
    /// This directory contains the session subdirectory with action logs and proof indices.
    pub fn persistence_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.persistence_dir = Some(dir.into());
        self
    }

    /// Set the event bus (required).
    pub fn event_bus(mut self, bus: EventBus) -> Self {
        self.event_bus = Some(bus);
        self
    }

    /// Set the oracle manager (required).
    pub fn oracles(mut self, oracles: OracleManager) -> Self {
        self.oracles = Some(oracles);
        self
    }

    /// Set optional directory to save proof files.
    ///
    /// If not set, proofs will only be indexed but not saved to disk.
    pub fn save_proofs_to(mut self, dir: impl Into<PathBuf>) -> Self {
        self.save_proofs_dir = Some(dir.into());
        self
    }

    /// Set the starting byte offset in the action log (optional, default: 0).
    ///
    /// Use this to resume proof generation from a checkpoint.
    pub fn start_offset(mut self, offset: u64) -> Self {
        self.start_offset = offset;
        self
    }

    /// Build the ProverWorker.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Required fields are missing
    /// - Repository creation fails
    /// - File I/O errors occur
    pub fn build(self) -> Result<ProverWorker, String> {
        use crate::repository::{FileActionLog, FileProofIndexRepository};

        // Validate required fields
        let session_id = self
            .session_id
            .ok_or_else(|| "session_id is required".to_string())?;
        let persistence_dir = self
            .persistence_dir
            .ok_or_else(|| "persistence_dir is required".to_string())?;
        let event_bus = self
            .event_bus
            .ok_or_else(|| "event_bus is required".to_string())?;
        let oracles = self
            .oracles
            .ok_or_else(|| "oracles is required".to_string())?;

        // Construct paths
        let session_dir = persistence_dir.join(&session_id);
        let action_filename = format!("actions_{}.log", session_id);
        let actions_dir = session_dir.join("actions");
        let proof_index_dir = session_dir.join("proof_indices");

        // Open action log repository
        let action_log = FileActionLog::open(&actions_dir, &action_filename)
            .map_err(|e| format!("Failed to open action log: {}", e))?;

        // Create proof index repository
        let proof_index_repo = FileProofIndexRepository::new(&proof_index_dir)
            .map_err(|e| format!("Failed to create proof index repository: {}", e))?;

        // Call the private constructor
        ProverWorker::new(
            Box::new(action_log),
            event_bus,
            oracles,
            self.save_proofs_dir,
            Box::new(proof_index_repo),
            session_id,
            self.start_offset,
        )
    }
}
