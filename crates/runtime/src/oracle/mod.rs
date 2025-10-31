//! Runtime wrappers around static game content oracles.
//!
//! These implementations expose `game-core` oracle traits and bundle them into
//! an [`OracleManager`] so the runtime can build [`game_core::Env`] snapshots
//! on demand. The data is immutable at runtime; dynamic state lives in
//! repositories or [`game_core::state::GameState`].
mod actors;
mod config;
mod items;
mod map;
mod tables;

use game_core::{Env, GameEnv, PcgRng};
use std::sync::Arc;

pub use actors::{ActorOracleImpl, AiConfig};
pub use config::ConfigOracleImpl;
pub use items::ItemOracleImpl;
pub use map::MapOracleImpl;
pub use tables::TablesOracleImpl;

/// Manages all oracle implementations and provides unified access
#[derive(Clone)]
pub struct OracleManager {
    pub(crate) map: Arc<MapOracleImpl>,
    pub(crate) items: Arc<ItemOracleImpl>,
    pub(crate) tables: Arc<TablesOracleImpl>,
    pub(crate) actors: Arc<ActorOracleImpl>,
    pub(crate) config: Arc<ConfigOracleImpl>,
    pub(crate) rng: PcgRng,
}

impl OracleManager {
    /// Creates a new oracle manager
    pub fn new(
        map: Arc<MapOracleImpl>,
        items: Arc<ItemOracleImpl>,
        tables: Arc<TablesOracleImpl>,
        actors: Arc<ActorOracleImpl>,
        config: Arc<ConfigOracleImpl>,
    ) -> Self {
        Self {
            map,
            items,
            tables,
            actors,
            config,
            rng: PcgRng, // PcgRng is stateless
        }
    }

    /// Converts oracle manager into GameEnv for game-core
    pub fn as_game_env(&self) -> GameEnv<'_> {
        Env::with_all(
            self.map.as_ref(),
            self.items.as_ref(),
            self.tables.as_ref(),
            self.actors.as_ref(),
            self.config.as_ref(),
            &self.rng,
        )
        .into_game_env()
    }

    /// Get access to actor oracle for runtime AI setup
    pub fn actors(&self) -> &ActorOracleImpl {
        &self.actors
    }
}
