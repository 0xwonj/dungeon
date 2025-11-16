//! Traits describing read-only world data.
//!
//! Oracles expose static map geometry, item definitions, rule tables, and NPC
//! templates. The [`Env`] aggregate bundles them so the engine can access
//! everything it needs without hard coupling to concrete implementations.
mod actions;
mod actors;
mod config;
mod error;
mod items;
mod map;
mod rng;
mod snapshot;

pub use actions::ActionOracle;
pub use actors::{ActorOracle, ActorTemplate, ActorTemplateBuilder};
pub use config::ConfigOracle;
pub use error::OracleError;
pub use items::{
    ArmorData, ConsumableData, ConsumableEffect, ItemDefinition, ItemKind, ItemOracle, WeaponData,
};
pub use map::{MapDimensions, MapOracle, StaticTile, TerrainKind};
pub use rng::{PcgRng, RngOracle, compute_seed};
pub use snapshot::{
    ActionSnapshot, ActorsSnapshot, ConfigSnapshot, ItemsSnapshot, MapSnapshot, OracleSnapshot,
    SnapshotActionOracle, SnapshotActorOracle, SnapshotConfigOracle, SnapshotItemOracle,
    SnapshotMapOracle, SnapshotOracleBundle,
};

/// Aggregates read-only oracles required by the reducer and action pipeline.
#[derive(Clone, Copy, Debug)]
pub struct Env<'a, M, I, T, A, C, R>
where
    M: MapOracle + ?Sized,
    I: ItemOracle + ?Sized,
    T: ActionOracle + ?Sized,
    A: ActorOracle + ?Sized,
    C: ConfigOracle + ?Sized,
    R: RngOracle + ?Sized,
{
    map: Option<&'a M>,
    items: Option<&'a I>,
    actions: Option<&'a T>,
    actors: Option<&'a A>,
    config: Option<&'a C>,
    rng: Option<&'a R>,
}

pub type GameEnv<'a> = Env<
    'a,
    dyn MapOracle + 'a,
    dyn ItemOracle + 'a,
    dyn ActionOracle + 'a,
    dyn ActorOracle + 'a,
    dyn ConfigOracle + 'a,
    dyn RngOracle + 'a,
>;

impl<'a, M, I, T, A, C, R> Env<'a, M, I, T, A, C, R>
where
    M: MapOracle + ?Sized,
    I: ItemOracle + ?Sized,
    T: ActionOracle + ?Sized,
    A: ActorOracle + ?Sized,
    C: ConfigOracle + ?Sized,
    R: RngOracle + ?Sized,
{
    pub fn new(
        map: Option<&'a M>,
        items: Option<&'a I>,
        actions: Option<&'a T>,
        actors: Option<&'a A>,
        config: Option<&'a C>,
        rng: Option<&'a R>,
    ) -> Self {
        Self {
            map,
            items,
            actions,
            actors,
            config,
            rng,
        }
    }

    pub fn with_all(
        map: &'a M,
        items: &'a I,
        actions: &'a T,
        actors: &'a A,
        config: &'a C,
        rng: &'a R,
    ) -> Self {
        Self::new(
            Some(map),
            Some(items),
            Some(actions),
            Some(actors),
            Some(config),
            Some(rng),
        )
    }

    pub fn empty() -> Self {
        Self {
            map: None,
            items: None,
            actions: None,
            actors: None,
            config: None,
            rng: None,
        }
    }

    /// Returns the MapOracle, or an error if not available.
    ///
    /// # Errors
    ///
    /// Returns `OracleError::MapNotAvailable` if no map oracle was provided.
    pub fn map(&self) -> Result<&'a M, OracleError> {
        self.map.ok_or(OracleError::MapNotAvailable)
    }

    /// Returns the ItemOracle, or an error if not available.
    ///
    /// # Errors
    ///
    /// Returns `OracleError::ItemsNotAvailable` if no items oracle was provided.
    pub fn items(&self) -> Result<&'a I, OracleError> {
        self.items.ok_or(OracleError::ItemsNotAvailable)
    }

    /// Returns the ActionOracle, or an error if not available.
    ///
    /// # Errors
    ///
    /// Returns `OracleError::ActionsNotAvailable` if no actions oracle was provided.
    pub fn actions(&self) -> Result<&'a T, OracleError> {
        self.actions.ok_or(OracleError::ActionsNotAvailable)
    }

    /// Returns the ActorOracle, or an error if not available.
    ///
    /// # Errors
    ///
    /// Returns `OracleError::ActorsNotAvailable` if no actors oracle was provided.
    pub fn actors(&self) -> Result<&'a A, OracleError> {
        self.actors.ok_or(OracleError::ActorsNotAvailable)
    }

    /// Returns the ConfigOracle, or an error if not available.
    ///
    /// # Errors
    ///
    /// Returns `OracleError::ConfigNotAvailable` if no config oracle was provided.
    pub fn config(&self) -> Result<&'a C, OracleError> {
        self.config.ok_or(OracleError::ConfigNotAvailable)
    }

    /// Returns the RngOracle, or an error if not available.
    ///
    /// # Errors
    ///
    /// Returns `OracleError::RngNotAvailable` if no rng oracle was provided.
    pub fn rng(&self) -> Result<&'a R, OracleError> {
        self.rng.ok_or(OracleError::RngNotAvailable)
    }

    /// Returns the activation radius from the config oracle.
    ///
    /// # Errors
    ///
    /// Returns `OracleError::ConfigNotAvailable` if no config oracle was provided.
    pub fn activation_radius(&self) -> Result<u32, OracleError> {
        Ok(self.config()?.activation_radius())
    }
}

impl<'a, M, I, T, A, C, R> Env<'a, M, I, T, A, C, R>
where
    M: MapOracle + 'a,
    I: ItemOracle + 'a,
    T: ActionOracle + 'a,
    A: ActorOracle + 'a,
    C: ConfigOracle + 'a,
    R: RngOracle + 'a,
{
    /// Converts this environment into a trait-object based `GameEnv` (consumes self).
    ///
    /// Use this when you need to convert once and don't need the original `Env` anymore.
    pub fn into_game_env(self) -> GameEnv<'a> {
        let map: Option<&'a dyn MapOracle> = self.map.map(|map| map as _);
        let items: Option<&'a dyn ItemOracle> = self.items.map(|items| items as _);
        let actions: Option<&'a dyn ActionOracle> = self.actions.map(|actions| actions as _);
        let actors: Option<&'a dyn ActorOracle> = self.actors.map(|actors| actors as _);
        let config: Option<&'a dyn ConfigOracle> = self.config.map(|config| config as _);
        let rng: Option<&'a dyn RngOracle> = self.rng.map(|rng| rng as _);
        Env::new(map, items, actions, actors, config, rng)
    }

    /// Converts this environment into a trait-object based `GameEnv` (borrows self).
    ///
    /// Use this when you need to convert multiple times (e.g., in a loop).
    /// Overhead: 6 pointer copies (48 bytes on 64-bit systems).
    pub fn as_game_env(&self) -> GameEnv<'a> {
        let map: Option<&'a dyn MapOracle> = self.map.map(|map| map as _);
        let items: Option<&'a dyn ItemOracle> = self.items.map(|items| items as _);
        let actions: Option<&'a dyn ActionOracle> = self.actions.map(|actions| actions as _);
        let actors: Option<&'a dyn ActorOracle> = self.actors.map(|actors| actors as _);
        let config: Option<&'a dyn ConfigOracle> = self.config.map(|config| config as _);
        let rng: Option<&'a dyn RngOracle> = self.rng.map(|rng| rng as _);
        Env::new(map, items, actions, actors, config, rng)
    }
}
