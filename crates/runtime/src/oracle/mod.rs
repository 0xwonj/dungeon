//! Runtime wrappers around static game content oracles.
//!
//! These implementations expose `game-core` oracle traits and bundle them into
//! an [`OracleBundle`] so the runtime can build [`game_core::Env`] snapshots
//! on demand. The data is immutable at runtime; dynamic state lives in
//! repositories or [`game_core::state::GameState`].
mod actions;
mod actors;
mod config;
mod items;
mod map;

use game_core::{Env, GameEnv, PcgRng};
use std::sync::Arc;

pub use actions::ActionOracleImpl;
pub use actors::ActorOracleImpl;
pub use config::ConfigOracleImpl;
pub use items::ItemOracleImpl;
pub use map::MapOracleImpl;

/// Bundle of oracle implementations that the runtime consumes.
///
/// This struct aggregates all oracle implementations and provides methods
/// to convert them into a `GameEnv` for use with game-core.
#[derive(Clone)]
pub struct OracleBundle {
    pub map: Arc<MapOracleImpl>,
    pub items: Arc<ItemOracleImpl>,
    pub actions: Arc<ActionOracleImpl>,
    pub actors: Arc<ActorOracleImpl>,
    pub config: Arc<ConfigOracleImpl>,
    rng: PcgRng,
}

impl OracleBundle {
    /// Creates a new oracle bundle
    pub fn new(
        map: Arc<MapOracleImpl>,
        items: Arc<ItemOracleImpl>,
        actions: Arc<ActionOracleImpl>,
        actors: Arc<ActorOracleImpl>,
        config: Arc<ConfigOracleImpl>,
    ) -> Self {
        Self {
            map,
            items,
            actions,
            actors,
            config,
            rng: PcgRng, // PcgRng is stateless
        }
    }

    /// Converts oracle bundle into GameEnv for game-core
    pub fn as_game_env(&self) -> GameEnv<'_> {
        Env::with_all(
            self.map.as_ref(),
            self.items.as_ref(),
            self.actions.as_ref(),
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

    /// Creates an oracle snapshot for zkVM execution.
    ///
    /// Captures all oracle data in a serializable format suitable for zkVM guests.
    /// Includes all actor templates available in the actor oracle.
    ///
    /// # Usage
    ///
    /// This snapshot is typically created once during runtime initialization and
    /// passed to the prover for all subsequent proof generation.
    pub fn to_snapshot(&self) -> zk::OracleSnapshot {
        use game_core::ActorOracle;

        let actor_ids = self.actors.all_ids();
        zk::OracleSnapshot::from_oracles(
            self.map.as_ref(),
            self.items.as_ref(),
            self.actors.as_ref(),
            self.actions.as_ref(),
            self.config.as_ref(),
            &actor_ids,
        )
    }
}
