//! High-level runtime orchestrator.
//!
//! The runtime owns background workers, wires up command/event channels, and
//! exposes a builder-based API for clients to drive the simulation.

use tokio::sync::{broadcast, mpsc};
use tokio::task::JoinHandle;

use game_core::{EntityId, GameConfig, GameState};

use crate::api::{ActionProvider, GameEvent, ProviderKind, Result, RuntimeError, RuntimeHandle};
use crate::oracle::OracleManager;
use crate::workers::{Command, SimulationWorker};

/// Runtime configuration shared across the orchestrator and workers.
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub game_config: GameConfig,
    pub event_buffer_size: usize,
    pub command_buffer_size: usize,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            game_config: GameConfig::default(),
            event_buffer_size: 100,
            command_buffer_size: 32,
        }
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
    // Future: proof_worker_handle, submit_worker_handle
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

    /// Subscribe to game events
    pub fn subscribe_events(&self) -> broadcast::Receiver<GameEvent> {
        self.handle.subscribe_events()
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
}

impl RuntimeBuilder {
    fn new() -> Self {
        Self {
            config: RuntimeConfig::default(),
            state: None,
            oracles: None,
            player_provider: None,
            npc_provider: None,
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
        let (event_tx, _event_rx) = broadcast::channel::<GameEvent>(self.config.event_buffer_size);

        let handle = RuntimeHandle::new(command_tx, event_tx.clone());

        let worker = SimulationWorker::new(
            initial_state,
            oracles,
            command_rx,
            event_tx.clone(),
        );

        let sim_worker_handle = tokio::spawn(async move {
            worker.run().await;
        });

        Ok(Runtime {
            handle,
            player_provider: self.player_provider,
            npc_provider: self.npc_provider,
            sim_worker_handle,
        })
    }
}
