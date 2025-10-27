//! ZK proof generation worker.
//!
//! This worker reads from the action log and generates zero-knowledge proofs
//! for executed actions. It maintains a cursor into the action log and processes
//! entries sequentially using the [`ActionLogReader`] trait.
//!
//! Design principles:
//! - Uses trait-based abstraction for flexible storage backends (mmap, in-memory, etc.)
//! - Default implementation uses memory-mapped files for zero-copy reading
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
    ActionLogEntry, ActionLogReader, ProofEntry, ProofIndex, ProofIndexRepository,
};
use crate::workers::ProofMetrics;

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
/// Reads ActionLogEntry records from the action log and generates zero-knowledge proofs.
pub struct ProverWorker {
    /// ZK prover instance
    prover: ZkProver,

    /// Action log reader (trait object for flexibility)
    reader: Box<dyn ActionLogReader>,

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
    fn new(
        reader: Box<dyn ActionLogReader>,
        event_bus: EventBus,
        oracle_manager: OracleManager,
        save_proofs_dir: Option<PathBuf>,
        proof_index_repo: Box<dyn ProofIndexRepository>,
        session_id: String,
    ) -> Result<Self, String> {
        // Create prover
        let oracle_snapshot = Self::create_oracle_snapshot(&oracle_manager);
        let prover = ZkProver::new(oracle_snapshot);

        // Load or create proof index
        let proof_index = match proof_index_repo.load(&session_id) {
            Ok(Some(index)) => {
                info!(
                    "Loaded existing proof index: session={}, proven_up_to={}, offset={}",
                    session_id, index.proven_up_to_nonce, index.action_log_offset
                );

                // Resume from checkpoint: seek reader to saved offset
                if index.action_log_offset > 0 {
                    if let Err(e) = reader.seek(index.action_log_offset) {
                        warn!(
                            "Failed to seek to checkpoint offset {}: {}. Starting from beginning.",
                            index.action_log_offset, e
                        );
                    } else {
                        info!(
                            "Resumed proof generation from offset {} (after nonce {})",
                            index.action_log_offset, index.proven_up_to_nonce
                        );
                    }
                }

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
            reader,
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
            ActorsSnapshot, ConfigSnapshot, ItemsSnapshot, MapSnapshot, OracleSnapshot,
            TablesSnapshot,
        };

        let map_snapshot = MapSnapshot::from_oracle(oracle_manager.map.as_ref());
        let items_snapshot = ItemsSnapshot::empty(); // TODO: Populate with actual items
        let actors_snapshot = ActorsSnapshot::empty(); // TODO: Populate with actual actors
        let tables_snapshot = TablesSnapshot::from_oracle(oracle_manager.tables.as_ref());
        let config_snapshot = ConfigSnapshot::from_oracle(oracle_manager.config.as_ref());

        OracleSnapshot::new(
            map_snapshot,
            items_snapshot,
            actors_snapshot,
            tables_snapshot,
            config_snapshot,
        )
    }

    /// Main worker loop.
    ///
    /// Continuously reads from the action log and generates proofs.
    ///
    /// # Performance Strategy
    ///
    /// The worker uses a tight loop for maximum throughput when there's backlog:
    /// 1. Read entries in a tight loop using the ActionLogReader trait
    /// 2. Process each entry immediately
    /// 3. Only when caught up: refresh and check for new data, sleep briefly if none
    ///
    /// This design assumes ProverWorker is always behind PersistenceWorker
    /// (proof generation is slower than action execution), so the worker
    /// spends most of its time in the tight loop with minimal overhead.
    ///
    /// The implementation is agnostic to the underlying storage mechanism -
    /// it works with any ActionLogReader (mmap, in-memory, S3, etc.).
    pub async fn run(mut self) {
        info!(
            "ProverWorker started (offset: {}, session: {})",
            self.reader.current_offset(),
            self.reader.session_id()
        );

        loop {
            // Tight loop: process all available entries
            loop {
                match self.reader.read_next() {
                    Ok(Some(entry)) => {
                        self.handle_action_entry(entry).await;
                    }
                    Ok(None) => {
                        // Caught up with writer - break to refresh
                        break;
                    }
                    Err(e) => {
                        // Handle read errors (partial writes, corrupted data, etc.)
                        error!(
                            "Failed to read action log at offset {}: {}",
                            self.reader.current_offset(),
                            e
                        );

                        // Check if this is a partial write error
                        if e.to_string().contains("partial write") {
                            error!(
                                "Detected partial write - action log may be corrupted. \
                                 Will retry after refresh."
                            );
                        }

                        // Break and attempt refresh - file may be still being written
                        break;
                    }
                }
            }

            // Caught up with writer - refresh and check for new data
            match self.reader.refresh() {
                Ok(true) => {
                    // New data available - immediately continue processing
                    debug!("Action log grew, resuming proof generation");
                    continue;
                }
                Ok(false) => {
                    // No new data - sleep briefly before checking again
                    time::sleep(Duration::from_millis(100)).await;
                }
                Err(e) => {
                    error!("Failed to refresh action log: {}", e);
                    time::sleep(Duration::from_secs(1)).await;
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
    /// 4. Update proof index
    async fn handle_action_entry(&mut self, entry: ActionLogEntry) {
        let nonce = entry.nonce;
        let clock = entry.clock;
        let action = entry.action.clone();
        let before_state = &*entry.before_state;
        let after_state = &*entry.after_state;

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
            .generate_proof(&action, before_state, after_state)
            .await
        {
            Ok((proof_data, proving_time)) => {
                let generation_time_ms = proving_time.as_millis() as u64;

                // Compute state hashes for logging
                use crate::utils::hash::hash_game_state;
                let before_hash = hash_game_state(before_state);
                let after_hash = hash_game_state(after_state);

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

                // Update action log offset for checkpoint/resume
                self.proof_index.action_log_offset = self.reader.current_offset();

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
    #[allow(dead_code)]
    start_offset: u64, // Deprecated: Use ProofIndex checkpoint instead
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

    /// Build the ProverWorker.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Required fields are missing
    /// - Repository creation fails
    /// - File I/O errors occur
    pub fn build(self) -> Result<ProverWorker, String> {
        use crate::repository::{FileProofIndexRepository, MmapActionLogReader};

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
        let action_log_path = session_dir.join("actions").join(&action_filename);
        let proof_index_dir = session_dir.join("proof_indices");

        // Create memory-mapped reader starting from beginning
        // (will be seeked to checkpoint offset in ProverWorker::new)
        let reader = MmapActionLogReader::new(action_log_path, session_id.clone(), 0)
            .map_err(|e| format!("Failed to create mmap reader: {}", e))?;

        // Box as trait object
        let reader: Box<dyn ActionLogReader> = Box::new(reader);

        // Create proof index repository
        let proof_index_repo = FileProofIndexRepository::new(&proof_index_dir)
            .map_err(|e| format!("Failed to create proof index repository: {}", e))?;

        // Call the private constructor
        ProverWorker::new(
            reader,
            event_bus,
            oracles,
            self.save_proofs_dir,
            Box::new(proof_index_repo),
            session_id,
        )
    }
}
