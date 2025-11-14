//! Prover worker for ZK proof generation.
//!
//! Monitors action batches and generates zero-knowledge proofs for completed batches.
//!
//! # Workflow
//!
//! 1. Poll for Complete action batches
//! 2. Load start state (previous batch's end state)
//! 3. Read all actions from the batch's action log
//! 4. Generate proof for the entire batch
//! 5. Save proof file and update batch status to Proven
//!
//! # Proof Generation Strategy
//!
//! Currently generates a single proof for the entire batch. Future optimization
//! could support incremental proving or parallel proof generation for large batches.

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use game_core::GameState;

use crate::repository::{
    ActionBatch, ActionBatchRepository, FileActionBatchRepository, FileActionLogReader,
    FileStateRepository, StateRepository,
};

use zk::{ProofData, Prover};

/// Result type for prover operations
pub type Result<T> = std::result::Result<T, ProverError>;

/// Configuration for the prover worker
#[derive(Debug, Clone)]
pub struct ProverConfig {
    /// Session identifier
    pub session_id: String,

    /// Base directory for all files
    pub base_dir: PathBuf,

    /// Maximum number of batches to prove in parallel
    pub max_parallel: usize,
}

impl ProverConfig {
    /// Create a new prover configuration
    pub fn new(session_id: String, base_dir: PathBuf) -> Self {
        Self {
            session_id,
            base_dir,
            max_parallel: 1,
        }
    }

    /// Set maximum number of parallel batch proofs
    pub fn with_max_parallel(mut self, max: usize) -> Self {
        self.max_parallel = max;
        self
    }
}

/// Commands that can be sent to the prover worker
#[allow(dead_code)]
pub enum Command {
    /// Prove all Complete batches from repository
    ProveBatches,

    /// Shutdown the worker gracefully
    Shutdown,
}

/// Background worker that generates ZK proofs for completed action batches
pub struct ProverWorker {
    config: ProverConfig,

    // Repositories (shared across parallel tasks)
    batch_repo: Arc<FileActionBatchRepository>,
    state_repo: Arc<FileStateRepository>,

    // Prover instance (shared across parallel tasks)
    prover: Arc<dyn Prover>,

    // Communication
    command_rx: mpsc::Receiver<Command>,
    batch_complete_rx: mpsc::Receiver<ActionBatch>,

    // Track running tasks to avoid blocking on completion
    running_tasks: Vec<tokio::task::JoinHandle<Result<()>>>,

    // Queue for batches when max_parallel is reached
    pending_batches: VecDeque<ActionBatch>,
}

impl ProverWorker {
    /// Create a new prover worker
    pub fn new(
        config: ProverConfig,
        prover: Arc<dyn Prover>,
        command_rx: mpsc::Receiver<Command>,
        batch_complete_rx: mpsc::Receiver<ActionBatch>,
    ) -> Result<Self> {
        let base_dir = &config.base_dir;
        let session_id = &config.session_id;

        // Create session directory
        let session_dir = base_dir.join(session_id);

        // Create repository instances
        let batch_repo = FileActionBatchRepository::new(session_dir.join("batches"))?;
        let state_repo = FileStateRepository::new(session_dir.join("states"))?;

        Ok(Self {
            config,
            batch_repo: Arc::new(batch_repo),
            state_repo: Arc::new(state_repo),
            prover,
            command_rx,
            batch_complete_rx,
            running_tasks: Vec::new(),
            pending_batches: VecDeque::new(),
        })
    }

    /// Main worker loop
    pub async fn run(mut self) {
        info!(
            "ProverWorker started: session={}, max_parallel={}",
            self.config.session_id, self.config.max_parallel
        );

        // Cleanup timer to prevent task accumulation
        let mut cleanup_interval = tokio::time::interval(tokio::time::Duration::from_secs(1));
        cleanup_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                // Handle completed batches from PersistenceWorker
                Some(batch) = self.batch_complete_rx.recv() => {
                    if let Err(e) = self.handle_completed_batch(batch).await {
                        error!("Failed to handle completed batch: {}", e);
                    }
                }

                cmd = self.command_rx.recv() => {
                    match cmd {
                        Some(Command::ProveBatches) => {
                            if let Err(e) = self.handle_prove_batches().await {
                                error!("Failed to prove batches: {}", e);
                            }
                        }
                        Some(Command::Shutdown) => {
                            info!("Shutdown command received");
                            break;
                        }
                        None => {
                            debug!("Command channel closed");
                            break;
                        }
                    }
                }

                // Periodic cleanup to prevent task accumulation and free slots
                _ = cleanup_interval.tick() => {
                    self.cleanup_completed_tasks();
                    // Process pending queue if slots freed up
                    self.process_pending_batches();
                }

                else => break,
            }
        }

        info!("ProverWorker stopped");
    }

    /// Handle a completed batch notification from PersistenceWorker
    async fn handle_completed_batch(&mut self, batch: ActionBatch) -> Result<()> {
        // Always queue the new batch first to maintain FIFO order
        self.pending_batches.push_back(batch);

        // Clean up completed tasks to free slots
        self.cleanup_completed_tasks();

        // Process queue in order (FIFO)
        self.process_pending_batches();

        Ok(())
    }

    /// Load all Complete batches from repository and queue them for proving
    async fn handle_prove_batches(&mut self) -> Result<()> {
        use crate::repository::ActionBatchStatus;

        info!("Loading Complete batches from repository...");

        // Load all Complete batches from repository
        let batches = self
            .batch_repo
            .list_by_status(&self.config.session_id, ActionBatchStatus::Complete)?;

        if batches.is_empty() {
            info!("No Complete batches found");
            return Ok(());
        }

        info!(
            "Found {} Complete batch(es), queuing for proving",
            batches.len()
        );

        // Queue all batches
        for batch in batches {
            self.pending_batches.push_back(batch);
        }

        // Clean up completed tasks to free slots
        self.cleanup_completed_tasks();

        // Process queue in order (FIFO)
        self.process_pending_batches();

        Ok(())
    }

    /// Process pending batches from the queue if slots are available
    fn process_pending_batches(&mut self) {
        while self.running_tasks.len() < self.config.max_parallel {
            if let Some(batch) = self.pending_batches.pop_front() {
                info!(
                    "Processing queued batch {} ({} remaining in queue)",
                    batch.start_nonce,
                    self.pending_batches.len()
                );
                self.spawn_proof_task(batch);
            } else {
                break;
            }
        }
    }

    /// Spawn a proof generation task for a batch
    fn spawn_proof_task(&mut self, batch: ActionBatch) {
        let start_nonce = batch.start_nonce;
        let config = self.config.clone();
        let batch_repo = Arc::clone(&self.batch_repo);
        let state_repo = Arc::clone(&self.state_repo);
        let prover = Arc::clone(&self.prover);

        // CRITICAL: Run entire proof generation in blocking thread pool
        // This prevents blocking tokio runtime with:
        // 1. CPU-intensive proof generation (RISC0 zkVM)
        // 2. Synchronous I/O operations (file reads/writes)
        let task = tokio::task::spawn_blocking(move || {
            if let Err(e) =
                Self::prove_batch_blocking(start_nonce, config, batch_repo, state_repo, prover)
            {
                error!("Proof generation failed for batch {}: {}", start_nonce, e);
                Err(e)
            } else {
                Ok(())
            }
        });

        self.running_tasks.push(task);

        info!(
            "Spawned proof task for batch {} ({}/{} slots used)",
            start_nonce,
            self.running_tasks.len(),
            self.config.max_parallel
        );
    }

    /// Clean up completed tasks from the running_tasks list
    fn cleanup_completed_tasks(&mut self) {
        let before = self.running_tasks.len();

        // Remove finished tasks and log any errors
        self.running_tasks.retain_mut(|task| {
            if task.is_finished() {
                // Task is done, log result if needed (can't access result without blocking)
                false // Remove from list
            } else {
                true // Keep in list
            }
        });

        let cleaned = before - self.running_tasks.len();
        if cleaned > 0 {
            debug!(
                "Cleaned up {} completed proof task(s), {}/{} slots now in use",
                cleaned,
                self.running_tasks.len(),
                self.config.max_parallel
            );
        }
    }

    /// Generate proof for a specific batch (blocking version)
    ///
    /// IMPORTANT: This function runs in a blocking thread pool and performs:
    /// 1. Synchronous file I/O (loading states, actions, saving proofs)
    /// 2. CPU-intensive proof generation (RISC0 zkVM execution)
    ///
    /// Never call this directly from async context - use spawn_blocking wrapper.
    fn prove_batch_blocking(
        start_nonce: u64,
        config: ProverConfig,
        batch_repo: Arc<FileActionBatchRepository>,
        state_repo: Arc<FileStateRepository>,
        prover: Arc<dyn Prover>,
    ) -> Result<()> {
        info!("Starting proof generation for batch {}", start_nonce);

        // Load batch metadata
        let mut batch = batch_repo
            .load(&config.session_id, start_nonce)?
            .ok_or(ProverError::BatchNotFound { start_nonce })?;

        // Check batch is in Complete state
        if !batch.is_ready_for_proving() {
            return Err(ProverError::BatchNotReady {
                start_nonce,
                status: batch.status,
            });
        }

        // Mark batch as Proving
        batch.mark_proving();
        batch_repo.save(&batch)?;

        // Load start state (genesis for batch 0, otherwise previous batch's end state)
        let start_state_nonce = if start_nonce == 0 { 0 } else { start_nonce - 1 };
        let start_state =
            state_repo
                .load(start_state_nonce)?
                .ok_or(ProverError::StateNotFound {
                    nonce: start_state_nonce,
                })?;

        // Load end state
        let end_state = state_repo
            .load(batch.end_nonce)?
            .ok_or(ProverError::StateNotFound {
                nonce: batch.end_nonce,
            })?;

        // Open action log reader
        let session_dir = config.base_dir.join(&config.session_id);
        let action_log_path = session_dir
            .join("actions")
            .join(batch.action_log_filename());

        let mut reader = FileActionLogReader::new(&action_log_path, config.session_id.clone())?;

        // Generate proof
        let proof_start = Instant::now();
        let proof_data =
            Self::generate_batch_proof(&batch, &start_state, &end_state, &mut reader, &prover)?;
        let generation_time_ms = proof_start.elapsed().as_millis() as u64;

        // Save proof file
        let proof_filename = batch.proof_filename();
        let proof_path = session_dir.join("proofs").join(&proof_filename);

        // Ensure proofs directory exists
        if let Some(parent) = proof_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Serialize and save proof
        let proof_bytes = bincode::serialize(&proof_data)?;
        std::fs::write(&proof_path, proof_bytes)?;

        // Update batch status to Proven
        batch.mark_proven(proof_filename, generation_time_ms);
        batch_repo.save(&batch)?;

        info!(
            "Proof generated for batch {}: {} actions in {}ms",
            start_nonce,
            batch.action_count(),
            generation_time_ms
        );

        Ok(())
    }

    /// Generate a proof for all actions in a batch (blocking version)
    ///
    /// Generates a single batch proof that verifies:
    /// - start_state + [action1, action2, ..., actionN] → end_state
    fn generate_batch_proof(
        batch: &ActionBatch,
        start_state: &GameState,
        end_state: &GameState,
        reader: &mut FileActionLogReader,
        prover: &Arc<dyn Prover>,
    ) -> Result<ProofData> {
        let action_count = batch.action_count();

        debug!(
            "Generating batch proof for {} action(s) in batch {}",
            action_count, batch.start_nonce
        );

        // Read all actions at once
        let entries = reader.read_all()?;

        if entries.is_empty() {
            return Err(ProverError::NoActions {
                start_nonce: batch.start_nonce,
            });
        }

        // Filter actions within this batch
        let batch_actions: Vec<_> = entries
            .into_iter()
            .filter(|entry| entry.nonce >= batch.start_nonce && entry.nonce <= batch.end_nonce)
            .map(|entry| entry.action)
            .collect();

        if batch_actions.len() != action_count as usize {
            warn!(
                "Action count mismatch: expected {}, found {}",
                action_count,
                batch_actions.len()
            );
            return Err(ProverError::ActionCountMismatch {
                start_nonce: batch.start_nonce,
                expected: action_count,
                actual: batch_actions.len(),
            });
        }

        // Generate proof: start_state + actions → end_state
        // NOTE: Already running in blocking thread pool (via spawn_blocking in spawn_proof_task)
        // so we can directly call the CPU-intensive prove() method here
        info!(
            "Starting proof generation for {} actions (batch {})",
            batch_actions.len(),
            batch.start_nonce
        );

        let proof = prover.prove(start_state, &batch_actions, end_state)?;

        info!(
            "Batch proof generated: {} actions, start_nonce={}, end_nonce={}",
            action_count, batch.start_nonce, batch.end_nonce
        );

        Ok(proof)
    }
}

/// Errors that can occur during proof generation
#[derive(Debug, thiserror::Error)]
pub enum ProverError {
    #[error("Batch {start_nonce} not found")]
    BatchNotFound { start_nonce: u64 },

    #[error("Batch {start_nonce} not ready for proving (status: {status:?})")]
    BatchNotReady {
        start_nonce: u64,
        status: crate::repository::ActionBatchStatus,
    },

    #[error("State not found at nonce {nonce}")]
    StateNotFound { nonce: u64 },

    #[error("No actions to prove in batch {start_nonce}")]
    NoActions { start_nonce: u64 },

    #[error("Action count mismatch in batch {start_nonce}: expected {expected}, found {actual}")]
    ActionCountMismatch {
        start_nonce: u64,
        expected: u64,
        actual: usize,
    },

    #[error(transparent)]
    Repository(#[from] crate::repository::RepositoryError),

    #[error(transparent)]
    Proof(#[from] zk::ProofError),

    #[error(transparent)]
    Serialization(#[from] bincode::Error),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}
