mod items;
mod map;
mod npc;
mod tables;

use game_core::{Env, GameEnv};
use std::sync::Arc;

pub use items::ItemOracleImpl;
pub use map::MapOracleImpl;
pub use npc::NpcOracleImpl;
pub use tables::TablesOracleImpl;

/// Manages all oracle implementations and provides unified access
pub struct OracleManager {
    pub(crate) map: Arc<MapOracleImpl>,
    pub(crate) items: Arc<ItemOracleImpl>,
    pub(crate) tables: Arc<TablesOracleImpl>,
    pub(crate) npcs: Arc<NpcOracleImpl>,
}

impl OracleManager {
    /// Creates a new oracle manager
    pub fn new(
        map: Arc<MapOracleImpl>,
        items: Arc<ItemOracleImpl>,
        tables: Arc<TablesOracleImpl>,
        npcs: Arc<NpcOracleImpl>,
    ) -> Self {
        Self {
            map,
            items,
            tables,
            npcs,
        }
    }

    /// Converts oracle manager into GameEnv for game-core
    pub fn as_game_env(&self) -> GameEnv<'_> {
        Env::with_all(
            self.map.as_ref(),
            self.items.as_ref(),
            self.tables.as_ref(),
            self.npcs.as_ref(),
        )
        .into_game_env()
    }
}
