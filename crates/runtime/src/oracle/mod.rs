//! Runtime wrappers around static game content oracles.
//!
//! These implementations expose `game-core` oracle traits and bundle them into
//! an [`OracleManager`] so the runtime can build [`game_core::Env`] snapshots
//! on demand. The data is immutable at runtime; dynamic state lives in
//! repositories or [`game_core::state::GameState`].
mod config;
mod items;
mod map;
mod npc;
mod tables;

use game_core::{Env, GameEnv};
use std::sync::Arc;

pub use config::ConfigOracleImpl;
pub use items::ItemOracleImpl;
pub use map::MapOracleImpl;
pub use npc::NpcOracleImpl;
pub use tables::TablesOracleImpl;

/// Manages all oracle implementations and provides unified access
#[derive(Clone)]
pub struct OracleManager {
    pub(crate) map: Arc<MapOracleImpl>,
    pub(crate) items: Arc<ItemOracleImpl>,
    pub(crate) tables: Arc<TablesOracleImpl>,
    pub(crate) npcs: Arc<NpcOracleImpl>,
    pub(crate) config: Arc<ConfigOracleImpl>,
}

impl OracleManager {
    /// Creates a new oracle manager
    pub fn new(
        map: Arc<MapOracleImpl>,
        items: Arc<ItemOracleImpl>,
        tables: Arc<TablesOracleImpl>,
        npcs: Arc<NpcOracleImpl>,
        config: Arc<ConfigOracleImpl>,
    ) -> Self {
        Self {
            map,
            items,
            tables,
            npcs,
            config,
        }
    }

    /// Converts oracle manager into GameEnv for game-core
    pub fn as_game_env(&self) -> GameEnv<'_> {
        Env::with_all(
            self.map.as_ref(),
            self.items.as_ref(),
            self.tables.as_ref(),
            self.npcs.as_ref(),
            self.config.as_ref(),
        )
        .into_game_env()
    }

    /// Creates a test oracle manager with default test data
    #[cfg(test)]
    pub fn test_manager() -> Self {
        Self::new(
            Arc::new(MapOracleImpl::test_map(20, 20)),
            Arc::new(ItemOracleImpl::test_items()),
            Arc::new(TablesOracleImpl::test_tables()),
            Arc::new(NpcOracleImpl::test_npcs()),
            Arc::new(ConfigOracleImpl::new(game_core::GameConfig::default())),
        )
    }
}
