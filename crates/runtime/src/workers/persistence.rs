//! Persistence worker for coordinated state and event persistence.
//!
//! Manages action batches, action log rotation, state snapshots, and event logging.
//!
//! # Checkpoint Strategy
//!
//! A "checkpoint" creates an action batch boundary:
//! 1. Current batch is marked Complete
//! 2. Current action log file is closed and rotated
//! 3. State at end_nonce is saved
//! 4. New batch is started with new action log file
//!
//! # State Management
//!
//! - Only end_nonce state is saved per batch
//! - ProverWorker loads previous batch's end state (= current batch's start state)
//! - First batch requires genesis state (nonce 0)
//!
//! # File Structure
//!
//! ```text
//! {base_dir}/{session_id}/
//!   ├── actions/
//!   │   ├── actions_{session}_{start}_{end}.log
//!   │   └── ...
//!   ├── batches/
//!   │   ├── batch_{end_nonce}.json
//!   │   └── ...
//!   ├── states/
//!   │   ├── state_{nonce}.bin
//!   │   └── ...
//!   └── events/
//!       └── events_{session}.log
//! ```

use std::path::PathBuf;

use tokio::sync::{broadcast, mpsc, oneshot};
use tokio::time::{Duration, sleep};
use tracing::{debug, error, info, warn};

use crate::events::{Event, GameStateEvent};
use crate::repository::{
    ActionBatch, ActionBatchRepository, ActionLogEntry, FileActionBatchRepository, FileActionLog,
    FileEventLog, FileStateRepository, StateRepository,
};
use crate::workers::simulation::Command as SimCommand;

/// Result type for persistence operations
pub type Result<T> = std::result::Result<T, PersistenceError>;

/// Errors that can occur during persistence operations
#[derive(Debug, thiserror::Error)]
pub enum PersistenceError {
    #[error("No active batch to checkpoint")]
    NoActiveBatch,

    #[error("Failed to save genesis state: {0}")]
    GenesisStateSave(String),

    #[error("Failed to save state at nonce {nonce}: {error}")]
    StateSave { nonce: u64, error: String },

    #[error("Failed to save batch: {0}")]
    BatchSave(String),

    #[error("Failed to append event: {0}")]
    EventAppend(String),

    #[error("Failed to flush event log: {0}")]
    EventFlush(String),

    #[error("Failed to append action log: {0}")]
    ActionLogAppend(String),

    #[error("Failed to flush action log: {0}")]
    ActionLogFlush(String),

    #[error("Failed to create action log: {0}")]
    ActionLogCreate(String),

    #[error("Failed to query state from SimulationWorker")]
    StateQuery,

    #[error("Failed to send command to SimulationWorker")]
    CommandSend,

    #[error(transparent)]
    Repository(#[from] crate::repository::RepositoryError),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// Checkpoint strategy determines when to create action batch boundaries
#[derive(Debug, Clone)]
pub enum CheckpointStrategy {
    /// Create checkpoint every N actions
    EveryNActions(u64),

    /// Manual checkpoints only (via command)
    Manual,
}

impl Default for CheckpointStrategy {
    fn default() -> Self {
        Self::EveryNActions(10)
    }
}

/// Configuration for the persistence worker
#[derive(Debug, Clone)]
pub struct PersistenceConfig {
    /// Session identifier (used for file naming)
    pub session_id: String,

    /// Base directory for all persistence files
    pub base_dir: PathBuf,

    /// Checkpoint strategy
    pub strategy: CheckpointStrategy,
}

impl PersistenceConfig {
    /// Create a new persistence configuration
    pub fn new(session_id: String, base_dir: PathBuf) -> Self {
        Self {
            session_id,
            base_dir,
            strategy: CheckpointStrategy::default(),
        }
    }

    /// Set checkpoint strategy
    pub fn with_strategy(mut self, strategy: CheckpointStrategy) -> Self {
        self.strategy = strategy;
        self
    }
}

/// Commands that can be sent to the persistence worker
#[allow(dead_code)]
pub enum Command {
    /// Manually trigger a checkpoint
    CreateCheckpoint { reply: oneshot::Sender<Result<u64>> },

    /// Shutdown the worker gracefully
    Shutdown,
}

/// Background worker that handles all persistence operations
pub struct PersistenceWorker {
    config: PersistenceConfig,

    // Repositories
    state_repo: FileStateRepository,
    batch_repo: FileActionBatchRepository,
    event_repo: FileEventLog,

    // Current batch tracking
    current_batch: Option<ActionBatch>,
    current_action_log: Option<FileActionLog>,

    // Communication channels
    event_rx: broadcast::Receiver<Event>,
    command_rx: mpsc::Receiver<Command>,
    sim_command_tx: mpsc::Sender<SimCommand>,
    batch_complete_tx: mpsc::Sender<ActionBatch>,

    // Checkpoint tracking
    strategy: CheckpointStrategy,
    actions_since_checkpoint: u64,
}

impl PersistenceWorker {
    /// Create a new persistence worker
    pub fn new(
        config: PersistenceConfig,
        event_rx: broadcast::Receiver<Event>,
        command_rx: mpsc::Receiver<Command>,
        sim_command_tx: mpsc::Sender<SimCommand>,
        batch_complete_tx: mpsc::Sender<ActionBatch>,
    ) -> Result<Self> {
        let base_dir = &config.base_dir;
        let session_id = &config.session_id;

        // Create session directory: base_dir/{session_id}/
        let session_dir = base_dir.join(session_id);

        // Create repository instances under session directory
        let state_repo = FileStateRepository::new(session_dir.join("states"))?;
        let batch_repo = FileActionBatchRepository::new(session_dir.join("batches"))?;

        let event_filename = format!("events_{}.log", session_id);
        let event_repo = FileEventLog::open_or_create(session_dir.join("events"), &event_filename)?;

        Ok(Self {
            strategy: config.strategy.clone(),
            config,
            state_repo,
            batch_repo,
            event_repo,
            current_batch: None,
            current_action_log: None,
            event_rx,
            command_rx,
            sim_command_tx,
            batch_complete_tx,
            actions_since_checkpoint: 0,
        })
    }

    /// Main worker loop
    pub async fn run(mut self) {
        info!(
            "PersistenceWorker started: session={}, strategy={:?}",
            self.config.session_id, self.strategy
        );

        // Save genesis state
        if let Err(e) = self.save_genesis_state().await {
            error!("Failed to save genesis state: {}", e);
            return;
        }

        // Start the first batch
        if let Err(e) = self.start_new_batch(0).await {
            error!("Failed to start initial batch: {}", e);
            return;
        }

        loop {
            tokio::select! {
                // Handle incoming events from event bus
                event = self.event_rx.recv() => {
                    match event {
                        Ok(event) => {
                            if let Err(e) = self.handle_event_with_retry(event).await {
                                panic!("FATAL persistence error: {}", e);
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(skipped)) => {
                            panic!(
                                "🚨 FATAL: PersistenceWorker lost {} events! \
                                 Data integrity COMPROMISED. This should NEVER happen with buffer size 50000. \
                                 Check system performance and disk I/O.",
                                skipped
                            );
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            info!("Event bus closed, shutting down PersistenceWorker");
                            break;
                        }
                    }
                }

                // Handle commands
                cmd = self.command_rx.recv() => {
                    match cmd {
                        Some(Command::CreateCheckpoint { reply }) => {
                            // For manual checkpoint, we need to query state
                            let (state_tx, state_rx) = oneshot::channel();
                            let result = if self.sim_command_tx.send(SimCommand::QueryState { reply: state_tx }).await.is_ok() {
                                if let Ok(state) = state_rx.await {
                                    self.create_checkpoint(&state).await
                                } else {
                                    Err(PersistenceError::StateQuery)
                                }
                            } else {
                                Err(PersistenceError::CommandSend)
                            };
                            let _ = reply.send(result);
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

                else => break,
            }
        }

        // Cleanup phase: flush all pending writes
        info!("Finalizing persistence worker...");
        if let Err(e) = self.finalize().await {
            error!("Failed to finalize: {}", e);
        }

        info!("PersistenceWorker stopped");
    }

    /// Save genesis state (nonce 0) at initialization
    async fn save_genesis_state(&mut self) -> Result<()> {
        debug!("Saving genesis state...");

        // Query current state from SimulationWorker
        let (reply_tx, reply_rx) = oneshot::channel();
        self.sim_command_tx
            .send(SimCommand::QueryState { reply: reply_tx })
            .await
            .map_err(|_| PersistenceError::CommandSend)?;

        let state = reply_rx.await.map_err(|_| PersistenceError::StateQuery)?;

        // Save genesis state at nonce 0
        self.state_repo
            .save(0, &state)
            .map_err(|e| PersistenceError::GenesisStateSave(e.to_string()))?;

        info!("Genesis state saved at nonce 0");
        Ok(())
    }

    /// Finalize persistence: flush all buffers and create final checkpoint
    async fn finalize(&mut self) -> Result<()> {
        debug!("Flushing all pending writes...");

        // Flush event log
        if let Err(e) = self.event_repo.flush() {
            error!("Failed to flush event log: {}", e);
        }

        // Check if there's an active batch with actions
        match &self.current_batch {
            Some(batch) if batch.action_count() > 0 => {
                let action_count = batch.action_count();
                let start_nonce = batch.start_nonce;

                info!(
                    "Creating final checkpoint for batch with {} action(s)",
                    action_count
                );

                // Query final state from SimulationWorker
                let (reply_tx, reply_rx) = oneshot::channel();
                self.sim_command_tx
                    .send(SimCommand::QueryState { reply: reply_tx })
                    .await
                    .map_err(|_| PersistenceError::CommandSend)?;

                let final_state = reply_rx.await.map_err(|_| PersistenceError::StateQuery)?;

                // Create final checkpoint
                if let Err(e) = self.create_checkpoint(&final_state).await {
                    error!("Failed to create final checkpoint: {}", e);
                } else {
                    info!(
                        "Final checkpoint created: batch {} complete with {} action(s)",
                        start_nonce, action_count
                    );
                }
            }
            Some(batch) => {
                // No actions in current batch, just flush and save
                let start_nonce = batch.start_nonce;

                if let Some(log) = self.current_action_log.as_mut() {
                    log.flush()
                        .map_err(|e| PersistenceError::ActionLogFlush(e.to_string()))?;
                }

                self.batch_repo
                    .save(batch)
                    .map_err(|e| PersistenceError::BatchSave(e.to_string()))?;

                debug!("Saved empty batch: start={}", start_nonce);
            }
            None => {
                debug!("No active batch to finalize");
            }
        }

        info!("Finalization complete");
        Ok(())
    }

    /// Handle an event with exponential backoff retry
    async fn handle_event_with_retry(&mut self, event: Event) -> Result<()> {
        const MAX_RETRIES: u32 = 5;
        const BASE_DELAY_MS: u64 = 100;

        let mut last_error = None;

        for attempt in 0..MAX_RETRIES {
            match self.handle_event(event.clone()).await {
                Ok(_) => {
                    if attempt > 0 {
                        info!("Event persisted successfully after {} retries", attempt);
                    }
                    return Ok(());
                }
                Err(e) => {
                    if attempt < MAX_RETRIES - 1 {
                        let delay = Duration::from_millis(BASE_DELAY_MS * (1 << attempt));
                        warn!(
                            "Failed to persist event (attempt {}/{}): {}. Retrying in {:?}...",
                            attempt + 1,
                            MAX_RETRIES,
                            e,
                            delay
                        );
                        sleep(delay).await;
                        last_error = Some(e);
                    } else {
                        error!(
                            "🚨 FATAL: Failed to persist event after {} attempts: {}",
                            MAX_RETRIES, e
                        );
                        return Err(e);
                    }
                }
            }
        }

        // This should never happen due to the return in the last attempt
        Err(last_error.unwrap())
    }

    /// Handle an event from the event bus
    async fn handle_event(&mut self, event: Event) -> Result<()> {
        match &event {
            Event::GameState(game_event) => {
                if let GameStateEvent::ActionExecuted {
                    nonce,
                    action,
                    after_state,
                    ..
                } = game_event
                {
                    self.handle_action_executed(*nonce, action.clone(), after_state)
                        .await?;
                }
            }
            _ => {
                // Save all events to event log
                self.event_repo
                    .append(&event)
                    .map_err(|e| PersistenceError::EventAppend(e.to_string()))?;

                self.event_repo
                    .flush()
                    .map_err(|e| PersistenceError::EventFlush(e.to_string()))?;
            }
        }

        Ok(())
    }

    /// Handle an executed action
    async fn handle_action_executed(
        &mut self,
        nonce: u64,
        action: game_core::Action,
        after_state: &game_core::GameState,
    ) -> Result<()> {
        // Ensure we have a current batch and action log
        if self.current_batch.is_none() || self.current_action_log.is_none() {
            return Err(PersistenceError::NoActiveBatch);
        }

        // Update batch end_nonce
        if let Some(batch) = self.current_batch.as_mut() {
            batch.end_nonce = nonce;
        }

        // Append to action log
        let entry = ActionLogEntry::new(nonce, action);
        if let Some(log) = self.current_action_log.as_mut() {
            log.append(&entry)
                .map_err(|e| PersistenceError::ActionLogAppend(e.to_string()))?;

            // Flush immediately for ProverWorker
            log.flush()
                .map_err(|e| PersistenceError::ActionLogFlush(e.to_string()))?;
        }

        self.actions_since_checkpoint += 1;

        debug!(
            "Persisted action: nonce={}, actor={:?}",
            nonce,
            entry.action.actor()
        );

        // Check if we should checkpoint
        if self.should_checkpoint()
            && let Err(e) = self.create_checkpoint(after_state).await
        {
            error!("Failed to create checkpoint: {}", e);
        }

        Ok(())
    }

    /// Check if we should create a checkpoint based on strategy
    fn should_checkpoint(&self) -> bool {
        match self.strategy {
            CheckpointStrategy::EveryNActions(n) => self.actions_since_checkpoint >= n,
            CheckpointStrategy::Manual => false,
        }
    }

    /// Create a checkpoint by completing current batch and starting new one
    ///
    /// The `state` parameter should be the after_state from the last ActionExecuted event.
    async fn create_checkpoint(&mut self, state: &game_core::GameState) -> Result<u64> {
        debug!("Creating checkpoint...");

        // Get current batch
        let mut batch = self
            .current_batch
            .take()
            .ok_or(PersistenceError::NoActiveBatch)?;

        let end_nonce = batch.end_nonce;

        // Save state at end_nonce
        self.state_repo
            .save(end_nonce, state)
            .map_err(|e| PersistenceError::StateSave {
                nonce: end_nonce,
                error: e.to_string(),
            })?;

        // Close current action log
        self.current_action_log = None;

        // Mark batch as complete
        batch.mark_complete(end_nonce);
        self.batch_repo
            .save(&batch)
            .map_err(|e| PersistenceError::BatchSave(e.to_string()))?;

        info!(
            "Checkpoint created: session={}, nonce={}, actions={}",
            self.config.session_id,
            end_nonce,
            batch.action_count()
        );

        // Notify ProverWorker about the completed batch
        // Use try_send (non-blocking) to avoid blocking game progress if ProverWorker is slow
        if let Err(e) = self.batch_complete_tx.try_send(batch.clone()) {
            warn!(
                "Failed to notify ProverWorker about completed batch (queue full or closed): {}. \
                 This is normal if proof generation is slower than game progress.",
                e
            );
        }

        // Start new batch
        self.start_new_batch(end_nonce + 1).await?;

        // Reset counter
        self.actions_since_checkpoint = 0;

        Ok(end_nonce)
    }

    /// Start a new action batch
    async fn start_new_batch(&mut self, start_nonce: u64) -> Result<()> {
        debug!("Starting new batch at nonce {}", start_nonce);

        // Create new batch
        let batch = ActionBatch::new(self.config.session_id.clone(), start_nonce);

        // Create new action log file
        let session_dir = self.config.base_dir.join(&self.config.session_id);
        let action_log_filename = batch.action_log_filename();
        let action_log =
            FileActionLog::open_or_create(session_dir.join("actions"), &action_log_filename)
                .map_err(|e| PersistenceError::ActionLogCreate(e.to_string()))?;

        // Save initial batch state
        self.batch_repo
            .save(&batch)
            .map_err(|e| PersistenceError::BatchSave(e.to_string()))?;

        self.current_batch = Some(batch);
        self.current_action_log = Some(action_log);

        info!(
            "New batch started: session={}, start_nonce={}",
            self.config.session_id, start_nonce
        );

        Ok(())
    }
}
