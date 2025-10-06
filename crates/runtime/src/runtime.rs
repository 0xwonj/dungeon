use tokio::sync::{broadcast, mpsc};
use tokio::task::JoinHandle;

use game_core::{GameConfig, GameState, MapDimensions};
use std::sync::Arc;

use crate::error::Result;
use crate::event::GameEvent;
use crate::handle::RuntimeHandle;
use crate::oracle::{ItemOracleImpl, MapOracleImpl, NpcOracleImpl, OracleManager, TablesOracleImpl};
use crate::repository::{InMemoryMapRepo, MapRepository};
use crate::worker::{Command, SimWorker};

/// Runtime configuration
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    /// Game configuration
    pub game_config: GameConfig,
    /// Event bus buffer size
    pub event_buffer_size: usize,
    /// Command queue buffer size
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

/// Main runtime that manages workers and provides handle to clients
pub struct Runtime {
    sim_worker_handle: JoinHandle<()>,
    event_tx: broadcast::Sender<GameEvent>,
}

impl Runtime {
    /// Start the runtime with given configuration
    /// Creates initial state from oracles using GameState::from_initial_entities()
    pub async fn start(config: RuntimeConfig) -> Result<RuntimeHandle> {
        // Create channels
        let (command_tx, command_rx) = mpsc::channel::<Command>(config.command_buffer_size);
        let (event_tx, _event_rx) = broadcast::channel::<GameEvent>(config.event_buffer_size);

        // Create repositories (for MVP, use test data)
        let map_repo = Arc::new(InMemoryMapRepo::test_map(10, 10)) as Arc<dyn MapRepository>;

        // Create oracles with initial entities
        let map_oracle = Arc::new(MapOracleImpl::test_map_with_entities(
            map_repo,
            MapDimensions::new(10, 10),
        ));
        let items_oracle = Arc::new(ItemOracleImpl::test_items());
        let tables_oracle = Arc::new(TablesOracleImpl::test_tables());
        let npcs_oracle = Arc::new(NpcOracleImpl::test_npcs());

        let oracles = OracleManager::new(map_oracle, items_oracle, tables_oracle, npcs_oracle);

        // Create initial state from oracles
        let env = oracles.as_game_env();
        let initial_state = GameState::from_initial_entities(&env)
            .map_err(|e| crate::error::RuntimeError::ExecuteFailed(format!("Failed to create initial state: {:?}", e)))?;

        // Spawn simulation worker
        let worker = SimWorker::new(
            initial_state,
            config.game_config,
            oracles,
            command_rx,
            event_tx.clone(),
        );

        let sim_worker_handle = tokio::spawn(async move {
            worker.run().await;
        });

        let _runtime = Runtime {
            sim_worker_handle,
            event_tx: event_tx.clone(),
        };

        // Create and return handle
        Ok(RuntimeHandle::new(command_tx, event_tx))
    }

    /// Shutdown the runtime gracefully
    pub async fn shutdown(self) -> Result<()> {
        // Drop command_tx to signal worker to stop
        drop(self.event_tx);

        // Wait for worker to finish
        self.sim_worker_handle
            .await
            .map_err(|e| crate::error::RuntimeError::ExecuteFailed(format!("Join error: {}", e)))?;

        Ok(())
    }
}
