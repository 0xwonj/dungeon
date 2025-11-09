//! High-level runtime orchestrator.
//!
//! The runtime owns background workers, wires up command/event channels, and
//! exposes a builder-based API for clients to drive the simulation.

use std::sync::{Arc, RwLock};

use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use game_core::{EntityId, GameConfig, GameState};

use crate::api::{
    ActionProvider, ProviderKind, ProviderRegistry, Result, RuntimeError, RuntimeHandle,
};
use crate::events::EventBus;
use crate::oracle::OracleManager;
use crate::providers::SystemActionProvider;
use crate::workers::{
    CheckpointStrategy, Command, PersistenceConfig, PersistenceWorker, ProofMetrics, ProverWorker,
    SimulationWorker,
};

/// Shared reference to proof generation metrics (thread-safe, lock-free)
type ProofMetricsArc = std::sync::Arc<ProofMetrics>;

/// Core runtime configuration for channels and buffers.
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub game_config: GameConfig,
    pub event_buffer_size: usize,
    pub command_buffer_size: usize,
    pub session_id: String,
}

/// Persistence worker configuration.
#[derive(Debug, Clone)]
pub struct PersistenceSettings {
    /// Enable persistence worker for state/event/proof persistence (default: false)
    pub enabled: bool,
    /// Base directory for persistence files (default: ./save_data)
    pub base_dir: std::path::PathBuf,
    /// Number of actions between automatic checkpoints (default: 10)
    pub checkpoint_interval: u64,
}

/// ZK proving worker configuration.
#[derive(Debug, Clone, Default)]
pub struct ProvingSettings {
    /// Enable ZK proof generation worker (default: false)
    pub enabled: bool,
    /// Optional directory to save generated proofs
    pub save_proofs_dir: Option<std::path::PathBuf>,
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
            session_id: format!("session_{}", timestamp),
        }
    }
}

impl Default for PersistenceSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            base_dir: Self::default_save_dir(),
            checkpoint_interval: 10,
        }
    }
}

impl PersistenceSettings {
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

/// Container for all background worker handles.
struct WorkerHandles {
    simulation: JoinHandle<()>,
    persistence: Option<JoinHandle<()>>,
    prover: Option<JoinHandle<()>>,
}

impl WorkerHandles {
    /// Shutdown all workers gracefully.
    ///
    /// Workers are shut down in order: simulation, persistence, then prover.
    async fn shutdown_all(self) -> Result<()> {
        // Simulation worker must be shut down first
        self.simulation.await.map_err(RuntimeError::WorkerJoin)?;

        // Then persistence worker
        if let Some(persistence_handle) = self.persistence {
            persistence_handle.await.map_err(RuntimeError::WorkerJoin)?;
        }

        // Finally prover worker
        if let Some(prover_handle) = self.prover {
            prover_handle.await.map_err(RuntimeError::WorkerJoin)?;
        }

        Ok(())
    }
}

/// Main runtime that orchestrates game simulation
///
/// Design: Runtime owns workers and coordinates execution.
/// [`RuntimeHandle`] provides a cloneable façade for clients.
pub struct Runtime {
    // Shared handle (can be cloned for clients)
    handle: RuntimeHandle,

    // Background workers (managed as a group)
    workers: WorkerHandles,

    // Proof metrics (shared with ProverWorker if enabled)
    // Uses atomics for lock-free access
    proof_metrics: Option<ProofMetricsArc>,

    // Provider registry (shared with RuntimeHandle via Arc)
    providers: Arc<RwLock<ProviderRegistry>>,

    // Oracle manager (cloned, cheap due to Arc internals)
    oracles: OracleManager,
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
    pub fn proof_metrics(&self) -> Option<ProofMetricsArc> {
        self.proof_metrics.as_ref().map(std::sync::Arc::clone)
    }

    /// Execute a single turn step.
    ///
    /// This is a high-level game loop helper that:
    /// 1. Prepares the next turn (determines which entity acts)
    /// 2. Queries the entity's provider for an action
    /// 3. Executes the action
    ///
    /// If the provider fails to generate an action, a fallback Wait action is used.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No active entities are available
    /// - The entity's provider kind is not registered
    /// - Action execution fails
    pub async fn step(&mut self) -> Result<()> {
        use game_core::{Action, ActionInput, ActionKind, CharacterAction};

        // 1. Prepare turn (SimulationWorker determines which entity acts)
        let (entity, snapshot) = self.handle.prepare_next_turn().await?;

        // 2. Get provider for this entity (from Runtime's registry)
        let provider = {
            let registry = self
                .providers
                .read()
                .map_err(|_| RuntimeError::LockPoisoned)?;
            registry.get_for_entity(entity)?
        };

        // 3. Query provider for action (I/O operation at Runtime layer)
        let env = self.oracles.as_game_env();
        let action = match provider.provide_action(entity, &snapshot, env).await {
            Ok(action) => action,
            Err(e) => {
                // Provider failed - log and fallback to Wait action
                tracing::warn!(
                    target: "runtime",
                    entity = ?entity,
                    error = %e,
                    "Provider failed to generate action, falling back to Wait"
                );
                Action::character(CharacterAction::new(
                    entity,
                    ActionKind::Wait,
                    ActionInput::None,
                ))
            }
        };

        // 4. Execute the action (SimulationWorker applies pure game logic)
        self.handle.execute_action(action).await?;

        Ok(())
    }

    /// Run the game loop continuously.
    ///
    /// This calls `step()` in a loop, automatically handling turn progression
    /// and action execution for all entities via their registered providers.
    pub async fn run(&mut self) -> Result<()> {
        loop {
            self.step().await?;
        }
    }

    /// Shutdown the runtime gracefully
    pub async fn shutdown(self) -> Result<()> {
        drop(self.handle);
        self.workers.shutdown_all().await
    }
}

/// Builder for [`Runtime`] with flexible configuration.
pub struct RuntimeBuilder {
    config: RuntimeConfig,
    persistence: PersistenceSettings,
    proving: ProvingSettings,
    state: Option<GameState>,
    oracles: Option<OracleManager>,
    scenario: Option<crate::scenario::Scenario>,
    providers: ProviderRegistry,
    system_provider: Option<SystemActionProvider>,
}

impl RuntimeBuilder {
    fn new() -> Self {
        Self {
            config: RuntimeConfig::default(),
            persistence: PersistenceSettings::default(),
            proving: ProvingSettings::default(),
            state: None,
            oracles: None,
            scenario: None,
            providers: ProviderRegistry::new(),
            system_provider: None,
        }
    }

    /// Override runtime configuration
    pub fn config(mut self, config: RuntimeConfig) -> Self {
        self.config = config;
        self
    }

    /// Override persistence settings
    pub fn persistence(mut self, settings: PersistenceSettings) -> Self {
        self.persistence = settings;
        self
    }

    /// Override proving settings
    pub fn proving(mut self, settings: ProvingSettings) -> Self {
        self.proving = settings;
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

    /// Provide scenario for entity initialization.
    ///
    /// If both scenario and initial_state are provided, initial_state takes precedence.
    pub fn scenario(mut self, scenario: crate::scenario::Scenario) -> Self {
        self.scenario = Some(scenario);
        self
    }

    /// Register a provider for a specific kind.
    pub fn provider(mut self, kind: ProviderKind, provider: impl ActionProvider + 'static) -> Self {
        self.providers.register(kind, provider);
        self
    }

    /// Bind an entity to a specific provider kind.
    pub fn entity_provider(mut self, entity: EntityId, kind: ProviderKind) -> Self {
        self.providers.bind_entity(entity, kind);
        self
    }

    /// Set the default provider kind for unmapped entities.
    pub fn default_provider(mut self, kind: ProviderKind) -> Self {
        self.providers.set_default(kind);
        self
    }

    /// Enable ZK proof generation worker
    pub fn enable_proving(mut self, enable: bool) -> Self {
        self.proving.enabled = enable;
        self
    }

    /// Enable persistence worker for state/event/proof persistence
    pub fn enable_persistence(mut self, enable: bool) -> Self {
        self.persistence.enabled = enable;
        self
    }

    /// Set session ID for persistence
    pub fn session_id(mut self, id: impl Into<String>) -> Self {
        self.config.session_id = id.into();
        self
    }

    /// Set base directory for persistence files
    pub fn persistence_dir(mut self, dir: impl Into<std::path::PathBuf>) -> Self {
        self.persistence.base_dir = dir.into();
        self
    }

    /// Set checkpoint interval (number of actions between checkpoints)
    pub fn checkpoint_interval(mut self, interval: u64) -> Self {
        self.persistence.checkpoint_interval = interval;
        self
    }

    /// Set custom event handlers.
    ///
    /// If not provided, the default handlers (ActionCost, Death, Activation) are used.
    /// Use this to add custom handlers or replace the default set entirely.
    /// Set a custom SystemActionProvider.
    ///
    /// By default, the runtime uses a provider with standard handlers (ActionCost, Death, Activation).
    /// Use this method to inject a custom provider with additional or modified handlers.
    pub fn with_system_provider(mut self, provider: SystemActionProvider) -> Self {
        self.system_provider = Some(provider);
        self
    }

    /// Validate the builder configuration.
    ///
    /// # Validation Rules
    ///
    /// - ZK proving requires persistence to be enabled
    /// - Session ID must not be empty
    /// - Buffer sizes must be greater than zero
    /// - Oracles must be provided
    fn validate(&self) -> Result<()> {
        // Proving requires persistence
        if self.proving.enabled && !self.persistence.enabled {
            return Err(RuntimeError::InvalidConfig(
                "ZK proving requires persistence to be enabled. \
                 Set enable_persistence(true) before enable_proving(true)."
                    .to_string(),
            ));
        }

        // Session ID validation
        if self.config.session_id.is_empty() {
            return Err(RuntimeError::InvalidConfig(
                "Session ID cannot be empty".to_string(),
            ));
        }

        // Buffer size validation
        if self.config.command_buffer_size == 0 {
            return Err(RuntimeError::InvalidConfig(
                "Command buffer size must be greater than 0".to_string(),
            ));
        }

        if self.config.event_buffer_size == 0 {
            return Err(RuntimeError::InvalidConfig(
                "Event buffer size must be greater than 0".to_string(),
            ));
        }

        // Persistence directory validation
        if self.persistence.enabled && !self.persistence.base_dir.as_os_str().is_empty() {
            // Basic check that the path is valid
            if self.persistence.base_dir.to_str().is_none() {
                return Err(RuntimeError::InvalidConfig(
                    "Persistence base directory contains invalid UTF-8".to_string(),
                ));
            }
        }

        // Checkpoint interval validation
        if self.persistence.enabled && self.persistence.checkpoint_interval == 0 {
            return Err(RuntimeError::InvalidConfig(
                "Checkpoint interval must be greater than 0".to_string(),
            ));
        }

        Ok(())
    }

    /// Build the runtime
    pub async fn build(self) -> Result<Runtime> {
        // Validate configuration before proceeding
        self.validate()?;

        // Extract all fields from self first to avoid partial move issues
        let RuntimeBuilder {
            config,
            persistence,
            proving,
            state,
            oracles,
            scenario,
            providers,
            system_provider,
        } = self;

        let oracles = oracles.ok_or_else(|| RuntimeError::MissingOracles)?;

        let initial_state = if let Some(state) = state {
            // Use provided state if available
            tracing::info!("Using provided initial state");
            state
        } else if let Some(scenario) = scenario {
            // Initialize from scenario
            tracing::info!("Initializing from scenario: {}", scenario.map_id);
            scenario.create_initial_state(&oracles)?
        } else {
            // No state or scenario provided - start with player state
            tracing::warn!("No scenario or initial state provided - using default with player");
            GameState::with_player()
        };

        let (command_tx, command_rx) = mpsc::channel::<Command>(config.command_buffer_size);
        let event_bus = EventBus::with_capacity(config.event_buffer_size);

        // Wrap providers in Arc<RwLock> for shared access
        let providers = Arc::new(RwLock::new(providers));

        let handle = RuntimeHandle::new(command_tx.clone(), event_bus.clone(), providers.clone());

        // Use provided system provider or default
        let system_provider = system_provider.unwrap_or_default();

        // Create workers using factory methods
        let sim_worker_handle = Self::create_simulation_worker(
            initial_state,
            oracles.clone(),
            command_rx,
            event_bus.clone(),
            system_provider,
        );

        let persistence_worker_handle = Self::create_persistence_worker(
            &config,
            &persistence,
            command_tx.clone(),
            event_bus.clone(),
        )?;

        let (prover_worker_handle, proof_metrics) = Self::create_prover_worker(
            &config,
            &persistence,
            &proving,
            event_bus.clone(),
            oracles.clone(),
        )?;

        Ok(Runtime {
            handle,
            workers: WorkerHandles {
                simulation: sim_worker_handle,
                persistence: persistence_worker_handle,
                prover: prover_worker_handle,
            },
            proof_metrics,
            providers,
            oracles,
        })
    }

    /// Create and spawn the simulation worker.
    fn create_simulation_worker(
        initial_state: GameState,
        oracles: OracleManager,
        command_rx: mpsc::Receiver<Command>,
        event_bus: EventBus,
        system_provider: SystemActionProvider,
    ) -> JoinHandle<()> {
        let sim_worker = SimulationWorker::new(
            initial_state,
            oracles,
            command_rx,
            event_bus,
            system_provider,
        );

        tokio::spawn(async move {
            sim_worker.run().await;
        })
    }

    /// Create and spawn the persistence worker (if enabled).
    fn create_persistence_worker(
        config: &RuntimeConfig,
        persistence: &PersistenceSettings,
        sim_command_tx: mpsc::Sender<Command>,
        event_bus: EventBus,
    ) -> Result<Option<JoinHandle<()>>> {
        if !persistence.enabled {
            return Ok(None);
        }

        let persistence_config =
            PersistenceConfig::new(config.session_id.clone(), persistence.base_dir.clone())
                .with_strategy(CheckpointStrategy::EveryNActions(
                    persistence.checkpoint_interval,
                ));

        let event_rx = event_bus.subscribe(crate::events::Topic::GameState);

        // PersistenceWorker has its own Command type, but we don't expose it
        // Create a dummy channel since we don't send commands to it yet
        let (_persistence_cmd_tx, persistence_cmd_rx) = mpsc::channel(8);

        let persistence_worker = PersistenceWorker::new(
            persistence_config,
            event_rx,
            persistence_cmd_rx,
            sim_command_tx,
        )
        .map_err(RuntimeError::InvalidConfig)?;

        let handle = tokio::spawn(async move {
            persistence_worker.run().await;
        });

        Ok(Some(handle))
    }

    /// Create and spawn the prover worker (if enabled).
    ///
    /// Returns the worker handle and metrics Arc, or None if proving is disabled.
    ///
    /// # Preconditions
    ///
    /// - Persistence must be enabled (validated in `RuntimeBuilder::validate()`)
    fn create_prover_worker(
        config: &RuntimeConfig,
        persistence: &PersistenceSettings,
        proving: &ProvingSettings,
        event_bus: EventBus,
        oracles: OracleManager,
    ) -> Result<(Option<JoinHandle<()>>, Option<ProofMetricsArc>)> {
        if !proving.enabled {
            return Ok((None, None));
        }

        // Persistence is guaranteed to be enabled by validate()
        debug_assert!(
            persistence.enabled,
            "Proving requires persistence - should be caught by validate()"
        );

        // Construct session directory path
        let session_dir = persistence.base_dir.join(&config.session_id);

        // Build ProverWorker using its builder
        // Note: ProverWorker automatically resumes from ProofIndex checkpoint
        let mut builder = ProverWorker::builder()
            .session_id(config.session_id.clone())
            .persistence_dir(&persistence.base_dir)
            .event_bus(event_bus)
            .oracles(oracles);

        // Optionally save proofs to disk
        if proving.save_proofs_dir.is_some() || persistence.enabled {
            let proofs_dir = session_dir.join("proofs");
            builder = builder.save_proofs_to(proofs_dir);
        }

        let prover_worker = builder.build().map_err(|e| {
            RuntimeError::InvalidConfig(format!("Failed to create ProverWorker: {}", e))
        })?;

        let prover_metrics = prover_worker.metrics();

        let handle = tokio::spawn(async move {
            prover_worker.run().await;
        });

        Ok((Some(handle), Some(prover_metrics)))
    }
}
