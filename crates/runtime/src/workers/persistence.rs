//! Persistence worker for coordinated state and event persistence.
//!
//! Subscribes to the event bus and persists game state, checkpoints, events,
//! and action logs according to configured checkpoint strategies.

use std::path::PathBuf;

use game_core::GameState;
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio::time::{Duration, sleep};
use tracing::{debug, error, info, warn};

use crate::events::{Event, GameStateEvent};
use crate::repository::{
    ActionLogEntry, Checkpoint, CheckpointRepository, FileActionLog, FileCheckpointRepository,
    FileEventLog, FileStateRepository, StateRepository,
};
use crate::workers::simulation::Command as SimCommand;

/// Checkpoint strategy determines when to save full state snapshots
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
    /// Session identifier (used for checkpoint and log naming)
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
#[allow(dead_code)] // Reserved for future manual checkpoint and graceful shutdown features
pub enum Command {
    /// Manually trigger a checkpoint
    CreateCheckpoint {
        reply: oneshot::Sender<Result<u64, String>>,
    },

    /// Shutdown the worker gracefully
    Shutdown,
}

/// Background worker that handles all persistence operations
pub struct PersistenceWorker {
    config: PersistenceConfig,

    // Repositories
    state_repo: FileStateRepository,
    checkpoint_repo: FileCheckpointRepository,
    event_repo: FileEventLog,
    action_repo: FileActionLog,

    // Communication channels
    event_rx: broadcast::Receiver<Event>,
    command_rx: mpsc::Receiver<Command>,
    sim_command_tx: mpsc::Sender<SimCommand>,

    // Checkpoint tracking
    strategy: CheckpointStrategy,
    actions_since_checkpoint: u64,
    last_checkpoint_nonce: u64,
}

impl PersistenceWorker {
    /// Create a new persistence worker
    pub fn new(
        config: PersistenceConfig,
        event_rx: broadcast::Receiver<Event>,
        command_rx: mpsc::Receiver<Command>,
        sim_command_tx: mpsc::Sender<SimCommand>,
    ) -> Result<Self, String> {
        let base_dir = &config.base_dir;
        let session_id = &config.session_id;

        // Create session directory: base_dir/{session_id}/
        let session_dir = base_dir.join(session_id);

        // Create repository instances under session directory
        let state_repo = FileStateRepository::new(session_dir.join("states"))
            .map_err(|e| format!("Failed to create state repository: {}", e))?;

        let checkpoint_repo = FileCheckpointRepository::new(session_dir.join("checkpoints"))
            .map_err(|e| format!("Failed to create checkpoint repository: {}", e))?;

        let event_filename = format!("events_{}.log", session_id);
        let event_repo = FileEventLog::open_or_create(session_dir.join("events"), &event_filename)
            .map_err(|e| format!("Failed to create event log: {}", e))?;

        let action_filename = format!("actions_{}.log", session_id);
        let action_repo =
            FileActionLog::open_or_create(session_dir.join("actions"), &action_filename)
                .map_err(|e| format!("Failed to create action log: {}", e))?;

        Ok(Self {
            strategy: config.strategy.clone(),
            config,
            state_repo,
            checkpoint_repo,
            event_repo,
            action_repo,
            event_rx,
            command_rx,
            sim_command_tx,
            actions_since_checkpoint: 0,
            last_checkpoint_nonce: 0,
        })
    }

    /// Main worker loop
    pub async fn run(mut self) {
        info!(
            "PersistenceWorker started: session={}, strategy={:?}",
            self.config.session_id, self.strategy
        );

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
                Some(cmd) = self.command_rx.recv() => {
                    match cmd {
                        Command::CreateCheckpoint { reply } => {
                            let result = self.create_checkpoint().await;
                            let _ = reply.send(result);
                        }
                        Command::Shutdown => {
                            info!("Shutdown command received");
                            break;
                        }
                    }
                }

                else => break,
            }
        }

        info!("PersistenceWorker stopped");
    }

    /// Handle an event with exponential backoff retry.
    ///
    /// Retries up to 5 times with exponential backoff (100ms, 200ms, 400ms, 800ms, 1600ms).
    /// Panics if all retries fail to ensure data integrity.
    async fn handle_event_with_retry(&mut self, event: Event) -> Result<(), String> {
        const MAX_RETRIES: u32 = 5;
        const BASE_DELAY_MS: u64 = 100;

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
                    } else {
                        error!(
                            "🚨 FATAL: Failed to persist event after {} attempts: {}",
                            MAX_RETRIES, e
                        );
                        return Err(format!(
                            "Persistence failed after {} retries: {}",
                            MAX_RETRIES, e
                        ));
                    }
                }
            }
        }

        unreachable!()
    }

    /// Handle an event from the event bus
    async fn handle_event(&mut self, event: Event) -> Result<(), String> {
        match &event {
            Event::GameState(game_event) => {
                if let GameStateEvent::ActionExecuted {
                    nonce,
                    action,
                    delta,
                    clock,
                    before_state,
                    after_state,
                    ..
                } = game_event
                {
                    // Save to action log (for proof generation)
                    let entry = ActionLogEntry {
                        nonce: *nonce,
                        clock: *clock,
                        action: action.clone(),
                        before_state: before_state.clone(),
                        after_state: after_state.clone(),
                        delta: Some(delta.clone()),
                    };

                    // Log state hashes for debugging chain consistency
                    use crate::utils::hash::hash_game_state;
                    debug!(
                        "ActionLog written at nonce {} | before_hash={} after_hash={}",
                        nonce,
                        &hash_game_state(before_state)[..8],
                        &hash_game_state(after_state)[..8]
                    );

                    self.action_repo
                        .append(&entry)
                        .map_err(|e| format!("Failed to append action log: {}", e))?;

                    // Flush immediately so ProverWorker can read it
                    self.action_repo
                        .flush()
                        .map_err(|e| format!("Failed to flush action log: {}", e))?;

                    self.actions_since_checkpoint += 1;

                    debug!(
                        "Persisted action: nonce={}, actor={:?}",
                        nonce,
                        action.actor()
                    );

                    // Check if we should checkpoint
                    if self.should_checkpoint()
                        && let Err(e) = self.create_checkpoint().await
                    {
                        error!("Failed to create checkpoint: {}", e);
                    }
                }
            }
            _ => {
                // Save all events to event log (for event replay)
                self.event_repo
                    .append(&event)
                    .map_err(|e| format!("Failed to append event: {}", e))?;

                // Flush for consistency (less critical than action log)
                self.event_repo
                    .flush()
                    .map_err(|e| format!("Failed to flush event log: {}", e))?;
            }
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

    /// Create a checkpoint by querying current state and saving it
    async fn create_checkpoint(&mut self) -> Result<u64, String> {
        debug!("Creating checkpoint...");

        // Query current state from SimulationWorker
        let (reply_tx, reply_rx) = oneshot::channel();
        self.sim_command_tx
            .send(SimCommand::QueryState { reply: reply_tx })
            .await
            .map_err(|e| format!("Failed to send QueryState command: {}", e))?;

        let state = reply_rx
            .await
            .map_err(|e| format!("Failed to receive state: {}", e))?;

        let nonce = state.turn.nonce;

        // Save state
        self.state_repo
            .save(nonce, &state)
            .map_err(|e| format!("Failed to save state: {}", e))?;

        // Create checkpoint
        let state_hash = self.compute_state_hash(&state);
        let action_offset = self.action_repo.size().unwrap_or(0);
        let checkpoint = Checkpoint::with_state(
            self.config.session_id.clone(),
            nonce,
            state_hash,
            true,
            action_offset,
        );

        debug!(
            "Checkpoint offsets: action={}, nonce={}",
            action_offset, nonce
        );

        self.checkpoint_repo
            .save(&checkpoint)
            .map_err(|e| format!("Failed to save checkpoint: {}", e))?;

        // Reset counters
        self.actions_since_checkpoint = 0;
        self.last_checkpoint_nonce = nonce;

        info!(
            "Checkpoint created: session={}, nonce={}",
            self.config.session_id, nonce
        );

        Ok(nonce)
    }

    /// Compute a hash of the game state for verification
    fn compute_state_hash(&self, state: &GameState) -> String {
        use crate::utils::hash::hash_game_state;
        hash_game_state(state)
    }
}
