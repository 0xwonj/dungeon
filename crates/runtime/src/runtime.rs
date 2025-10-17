//! High-level runtime orchestrator.
//!
//! The runtime owns background workers, wires up command/event channels, and
//! exposes a builder-based API for clients to drive the simulation.

use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use game_core::{EntityId, GameConfig, GameState};

use crate::api::{ActionProvider, ProviderKind, Result, RuntimeError, RuntimeHandle};
use crate::events::EventBus;
use crate::hooks::{HookRegistry, PostExecutionHook};
use crate::oracle::OracleManager;
use crate::workers::{
    CheckpointStrategy, Command, PersistenceConfig, PersistenceWorker, ProofMetrics, ProverWorker,
    SimulationWorker,
};

/// Runtime configuration shared across the orchestrator and workers.
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub game_config: GameConfig,
    pub event_buffer_size: usize,
    pub command_buffer_size: usize,
    /// Enable ZK proof generation worker (default: false)
    pub enable_proving: bool,
    /// Optional directory to save generated proofs
    pub save_proofs_dir: Option<std::path::PathBuf>,
    /// Enable persistence worker for state/event/proof persistence (default: false)
    pub enable_persistence: bool,
    /// Base directory for persistence files (default: ./save_data)
    pub persistence_base_dir: std::path::PathBuf,
    /// Session ID for this runtime instance
    pub session_id: String,
    /// Number of actions between automatic checkpoints (default: 10)
    pub checkpoint_interval: u64,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            game_config: GameConfig::default(),
            event_buffer_size: 100,
            command_buffer_size: 32,
            enable_proving: false,
            save_proofs_dir: None,
            enable_persistence: false,
            persistence_base_dir: Self::default_save_dir(),
            session_id: format!("session_{}", timestamp),
            checkpoint_interval: 10,
        }
    }
}

impl RuntimeConfig {
    /// Get the default system directory for save data
    ///
    /// Uses platform-specific directories via the `directories` crate:
    /// - macOS: `~/Library/Application Support/dungeon`
    /// - Linux: `~/.local/share/dungeon` (or `$XDG_DATA_HOME/dungeon`)
    /// - Windows: `%APPDATA%\dungeon`
    /// - Fallback: `./save_data` (current directory)
    fn default_save_dir() -> std::path::PathBuf {
        directories::ProjectDirs::from("", "", "dungeon")
            .map(|dirs| dirs.data_dir().to_path_buf())
            .unwrap_or_else(|| std::path::PathBuf::from("./save_data"))
    }
}

/// Main runtime that orchestrates game simulation
///
/// Design: Runtime owns workers and coordinates execution.
/// [`RuntimeHandle`] provides a cloneable façade for clients.
pub struct Runtime {
    // Shared handle (can be cloned for clients)
    handle: RuntimeHandle,

    // Action providers (injected by user)
    player_provider: Option<Box<dyn ActionProvider>>,
    npc_provider: Option<Box<dyn ActionProvider>>,

    // Background workers
    sim_worker_handle: JoinHandle<()>,
    prover_worker_handle: Option<JoinHandle<()>>,
    persistence_worker_handle: Option<JoinHandle<()>>,

    // Proof metrics (shared with ProverWorker if enabled)
    // Uses atomics for lock-free access
    proof_metrics: Option<std::sync::Arc<ProofMetrics>>,
}

impl Runtime {
    /// Create a new runtime builder
    pub fn builder() -> RuntimeBuilder {
        RuntimeBuilder::new()
    }

    /// Get a cloneable handle to this runtime
    ///
    /// The handle can be shared across clients and async tasks.
    pub fn handle(&self) -> RuntimeHandle {
        self.handle.clone()
    }

    /// Get proof generation metrics Arc (if proving is enabled).
    ///
    /// Returns `None` if proving is not enabled.
    /// The returned Arc can be used to query metrics without locking.
    pub fn proof_metrics(&self) -> Option<std::sync::Arc<ProofMetrics>> {
        self.proof_metrics.as_ref().map(std::sync::Arc::clone)
    }

    /// Execute a single turn step
    ///
    /// Requires both player and NPC providers to be configured.
    pub async fn step(&mut self) -> Result<()> {
        let player_provider =
            self.player_provider
                .as_ref()
                .ok_or_else(|| RuntimeError::ProviderNotSet {
                    kind: ProviderKind::Player,
                })?;
        let npc_provider =
            self.npc_provider
                .as_ref()
                .ok_or_else(|| RuntimeError::ProviderNotSet {
                    kind: ProviderKind::Npc,
                })?;

        let (entity, snapshot) = self.handle.prepare_next_turn().await?;

        let action = if entity == EntityId::PLAYER {
            player_provider.provide_action(entity, &snapshot).await?
        } else {
            npc_provider.provide_action(entity, &snapshot).await?
        };

        self.handle.execute_action(action).await?;

        Ok(())
    }

    /// Run the game loop continuously
    pub async fn run(&mut self) -> Result<()> {
        loop {
            self.step().await?;
        }
    }

    /// Set the player action provider
    pub fn set_player_provider(&mut self, provider: impl ActionProvider + 'static) {
        self.player_provider = Some(Box::new(provider));
    }

    /// Set the NPC action provider
    pub fn set_npc_provider(&mut self, provider: impl ActionProvider + 'static) {
        self.npc_provider = Some(Box::new(provider));
    }

    /// Shutdown the runtime gracefully
    pub async fn shutdown(self) -> Result<()> {
        drop(self.handle);

        self.sim_worker_handle
            .await
            .map_err(RuntimeError::WorkerJoin)?;

        if let Some(persistence_handle) = self.persistence_worker_handle {
            persistence_handle.await.map_err(RuntimeError::WorkerJoin)?;
        }

        if let Some(prover_handle) = self.prover_worker_handle {
            prover_handle.await.map_err(RuntimeError::WorkerJoin)?;
        }

        Ok(())
    }
}

/// Builder for [`Runtime`] with flexible configuration.
pub struct RuntimeBuilder {
    config: RuntimeConfig,
    state: Option<GameState>,
    oracles: Option<OracleManager>,
    player_provider: Option<Box<dyn ActionProvider>>,
    npc_provider: Option<Box<dyn ActionProvider>>,
    hooks: Option<HookRegistry>,
}

impl RuntimeBuilder {
    fn new() -> Self {
        Self {
            config: RuntimeConfig::default(),
            state: None,
            oracles: None,
            player_provider: None,
            npc_provider: None,
            hooks: None,
        }
    }

    /// Override runtime configuration
    pub fn config(mut self, config: RuntimeConfig) -> Self {
        self.config = config;
        self
    }

    /// Provide initial game state
    pub fn initial_state(mut self, state: GameState) -> Self {
        self.state = Some(state);
        self
    }

    /// Set required oracle manager
    pub fn oracles(mut self, oracles: OracleManager) -> Self {
        self.oracles = Some(oracles);
        self
    }

    /// Set player action provider (optional)
    pub fn player_provider(mut self, provider: impl ActionProvider + 'static) -> Self {
        self.player_provider = Some(Box::new(provider));
        self
    }

    /// Set NPC action provider (optional)
    pub fn npc_provider(mut self, provider: impl ActionProvider + 'static) -> Self {
        self.npc_provider = Some(Box::new(provider));
        self
    }

    /// Enable ZK proof generation worker
    pub fn enable_proving(mut self, enable: bool) -> Self {
        self.config.enable_proving = enable;
        self
    }

    /// Enable persistence worker for state/event/proof persistence
    pub fn enable_persistence(mut self, enable: bool) -> Self {
        self.config.enable_persistence = enable;
        self
    }

    /// Set session ID for persistence
    pub fn session_id(mut self, id: impl Into<String>) -> Self {
        self.config.session_id = id.into();
        self
    }

    /// Set base directory for persistence files
    pub fn persistence_dir(mut self, dir: impl Into<std::path::PathBuf>) -> Self {
        self.config.persistence_base_dir = dir.into();
        self
    }

    /// Set checkpoint interval (number of actions between checkpoints)
    pub fn checkpoint_interval(mut self, interval: u64) -> Self {
        self.config.checkpoint_interval = interval;
        self
    }

    /// Set custom post-execution hooks.
    ///
    /// If not provided, the default hooks (ActionCost, Activation) are used.
    /// Use this to add custom hooks or replace the default set entirely.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use std::sync::Arc;
    ///
    /// let root_hooks = vec![
    ///     Arc::new(ActionCostHook) as Arc<dyn PostExecutionHook>,
    ///     Arc::new(ActivationHook) as Arc<dyn PostExecutionHook>,
    ///     Arc::new(DamageHook) as Arc<dyn PostExecutionHook>,
    /// ];
    ///
    /// let all_hooks = vec![
    ///     Arc::new(ActionCostHook) as Arc<dyn PostExecutionHook>,
    ///     Arc::new(ActivationHook) as Arc<dyn PostExecutionHook>,
    ///     Arc::new(DamageHook) as Arc<dyn PostExecutionHook>,
    ///     Arc::new(DeathCheckHook) as Arc<dyn PostExecutionHook>, // Lookup only
    /// ];
    ///
    /// let runtime = Runtime::builder()
    ///     .with_hooks(HookRegistry::new(root_hooks, all_hooks))
    ///     .build()
    ///     .await?;
    /// ```
    pub fn with_hooks(mut self, hooks: HookRegistry) -> Self {
        self.hooks = Some(hooks);
        self
    }

    /// Adds hooks to the default hook set.
    ///
    /// This is a convenience method for adding custom hooks without replacing
    /// the entire default set.
    ///
    /// # Arguments
    ///
    /// * `root_hooks` - Additional hooks to execute on every action
    /// * `lookup_hooks` - Additional hooks that are only chained (not executed as root)
    ///
    /// Note: If you've already called `with_hooks()`, calling this will discard
    /// those hooks and rebuild from the default set plus your new hooks.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use std::sync::Arc;
    ///
    /// let runtime = Runtime::builder()
    ///     .add_hooks(
    ///         vec![Arc::new(DamageHook)],  // Root: execute every action
    ///         vec![Arc::new(DeathCheckHook)],  // Lookup: only when chained
    ///     )
    ///     .build()
    ///     .await?;
    /// ```
    pub fn add_hooks(
        mut self,
        additional_root_hooks: Vec<std::sync::Arc<dyn PostExecutionHook>>,
        additional_lookup_hooks: Vec<std::sync::Arc<dyn PostExecutionHook>>,
    ) -> Self {
        use crate::hooks::{ActionCostHook, ActivationHook};
        use std::sync::Arc;

        // Start with default hooks
        let mut root_hooks: Vec<Arc<dyn PostExecutionHook>> = vec![
            Arc::new(ActionCostHook) as Arc<dyn PostExecutionHook>,
            Arc::new(ActivationHook) as Arc<dyn PostExecutionHook>,
        ];
        root_hooks.extend(additional_root_hooks);

        // All hooks = root + lookup-only
        let mut all_hooks = root_hooks.clone();
        all_hooks.extend(additional_lookup_hooks);

        self.hooks = Some(HookRegistry::new(root_hooks, all_hooks));
        self
    }

    /// Build the runtime
    pub async fn build(self) -> Result<Runtime> {
        let oracles = self.oracles.ok_or_else(|| RuntimeError::MissingOracles)?;

        let initial_state = if let Some(state) = self.state {
            state
        } else {
            let env = oracles.as_game_env();
            GameState::from_initial_entities(&env).map_err(RuntimeError::InitialState)?
        };

        let (command_tx, command_rx) = mpsc::channel::<Command>(self.config.command_buffer_size);
        let event_bus = EventBus::with_capacity(self.config.event_buffer_size);

        let handle = RuntimeHandle::new(command_tx.clone(), event_bus.clone());
        let command_tx_for_persistence = command_tx.clone();

        // Use provided hooks or default registry
        let hooks = self.hooks.unwrap_or_default();

        // Create simulation worker
        let sim_worker = SimulationWorker::new(
            initial_state.clone(),
            oracles.clone(),
            command_rx,
            event_bus.clone(),
            hooks,
        );

        let sim_worker_handle = tokio::spawn(async move {
            sim_worker.run().await;
        });

        // Create persistence worker (if enabled)
        let persistence_worker_handle = if self.config.enable_persistence {
            let persistence_config = PersistenceConfig::new(
                self.config.session_id.clone(),
                self.config.persistence_base_dir.clone(),
            )
            .with_strategy(CheckpointStrategy::EveryNActions(
                self.config.checkpoint_interval,
            ));

            let event_rx = event_bus.subscribe(crate::events::Topic::GameState);

            // PersistenceWorker has its own Command type, but we don't expose it
            // Create a dummy channel since we don't send commands to it yet
            let (_persistence_cmd_tx, persistence_cmd_rx) = mpsc::channel(8);

            let persistence_worker = PersistenceWorker::new(
                persistence_config,
                event_rx,
                persistence_cmd_rx,
                command_tx_for_persistence.clone(),
            )
            .map_err(RuntimeError::InvalidConfig)?;

            Some(tokio::spawn(async move {
                persistence_worker.run().await;
            }))
        } else {
            None
        };

        // Create prover worker (if enabled and persistence is enabled)
        let (prover_worker_handle, proof_metrics) = if self.config.enable_proving {
            if !self.config.enable_persistence {
                return Err(RuntimeError::InvalidConfig(
                    "ZK proving requires persistence to be enabled. \
                     Set enable_persistence(true) before enable_proving(true)."
                        .to_string(),
                ));
            } else {
                use crate::repository::FileActionLog;

                // Open the action log file that PersistenceWorker is writing to
                let session_dir = self
                    .config
                    .persistence_base_dir
                    .join(&self.config.session_id);
                let action_filename = format!("actions_{}.log", self.config.session_id);
                let action_log = FileActionLog::open(session_dir.join("actions"), &action_filename)
                    .map_err(|e| {
                        RuntimeError::InvalidConfig(format!("Failed to open action log: {}", e))
                    })?;

                // Proof index directory (dedicated subdirectory)
                let proof_index_dir = session_dir.join("proof_indices");

                // Proofs directory (dedicated subdirectory)
                let proofs_dir = session_dir.join("proofs");

                let prover_worker = ProverWorker::new(
                    action_log,
                    event_bus.clone(),
                    oracles.clone(),
                    Some(proofs_dir),
                    proof_index_dir,
                    self.config.session_id.clone(),
                    0, // start_offset
                )
                .map_err(|e| {
                    RuntimeError::InvalidConfig(format!("Failed to create ProverWorker: {}", e))
                })?;

                let prover_metrics = prover_worker.metrics();

                let handle = tokio::spawn(async move {
                    prover_worker.run().await;
                });

                (Some(handle), Some(prover_metrics))
            }
        } else {
            (None, None)
        };

        Ok(Runtime {
            handle,
            player_provider: self.player_provider,
            npc_provider: self.npc_provider,
            sim_worker_handle,
            prover_worker_handle,
            persistence_worker_handle,
            proof_metrics,
        })
    }
}
