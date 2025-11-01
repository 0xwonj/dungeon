//! Traits describing read-only world data.
//!
//! Oracles expose static map geometry, item definitions, rule tables, and NPC
//! templates. The [`Env`] aggregate bundles them so the engine can access
//! everything it needs without hard coupling to concrete implementations.
mod config;
mod items;
mod map;
mod npc;
mod rng;
mod snapshot;
mod tables;

pub use config::ConfigOracle;
pub use items::{
    ArmorData, ConsumableData, ConsumableEffect, ItemDefinition, ItemKind, ItemOracle, WeaponData,
};
pub use map::{MapDimensions, MapOracle, StaticTile, TerrainKind};
pub use npc::{ActorOracle, ActorTemplate, ActorTemplateBuilder};
pub use rng::{PcgRng, RngOracle, compute_seed};
pub use snapshot::{
    ActorsSnapshot, ConfigSnapshot, ItemsSnapshot, MapSnapshot, OracleSnapshot,
    SnapshotActorOracle, SnapshotConfigOracle, SnapshotItemOracle, SnapshotMapOracle,
    SnapshotOracleBundle, SnapshotTablesOracle, TablesSnapshot,
};
pub use tables::{
    ActionCosts, CombatParams, DamageParams, HitChanceParams, SpeedParams, TablesOracle,
};

use crate::error::{ErrorSeverity, GameError};
use crate::state::{ItemHandle, Position};

/// Errors that occur when accessing Oracle data.
///
/// Oracle errors indicate that required game data is unavailable or invalid.
/// These are typically fatal errors since the game engine cannot proceed without
/// access to maps, items, or balance tables.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum OracleError {
    /// MapOracle is not available in the environment.
    #[error("MapOracle not available")]
    MapNotAvailable,

    /// ItemOracle is not available in the environment.
    #[error("ItemOracle not available")]
    ItemsNotAvailable,

    /// ActorOracle is not available in the environment.
    #[error("ActorOracle not available")]
    ActorsNotAvailable,

    /// TablesOracle is not available in the environment.
    #[error("TablesOracle not available")]
    TablesNotAvailable,

    /// ConfigOracle is not available in the environment.
    #[error("ConfigOracle not available")]
    ConfigNotAvailable,

    /// RngOracle is not available in the environment.
    #[error("RngOracle not available")]
    RngNotAvailable,

    /// Position is outside the map bounds.
    #[error("position {0:?} is out of map bounds")]
    PositionOutOfBounds(Position),

    /// Tile at the given position was not found.
    #[error("tile at position {0:?} not found")]
    TileNotFound(Position),

    /// Item definition was not found by handle.
    #[error("item definition {0:?} not found")]
    ItemNotFound(ItemHandle),

    /// Actor template was not found by ID.
    #[error("actor template '{0}' not found")]
    ActorTemplateNotFound(&'static str),
}

impl GameError for OracleError {
    fn severity(&self) -> ErrorSeverity {
        use OracleError::*;
        match self {
            // Missing oracles are fatal - engine cannot proceed
            MapNotAvailable | ItemsNotAvailable | ActorsNotAvailable | TablesNotAvailable
            | ConfigNotAvailable | RngNotAvailable => ErrorSeverity::Fatal,

            // Not found errors are validation errors - invalid references
            PositionOutOfBounds(_)
            | TileNotFound(_)
            | ItemNotFound(_)
            | ActorTemplateNotFound(_) => ErrorSeverity::Validation,
        }
    }

    fn error_code(&self) -> &'static str {
        use OracleError::*;
        match self {
            MapNotAvailable => "ORACLE_MAP_NOT_AVAILABLE",
            ItemsNotAvailable => "ORACLE_ITEMS_NOT_AVAILABLE",
            ActorsNotAvailable => "ORACLE_ACTORS_NOT_AVAILABLE",
            TablesNotAvailable => "ORACLE_TABLES_NOT_AVAILABLE",
            ConfigNotAvailable => "ORACLE_CONFIG_NOT_AVAILABLE",
            RngNotAvailable => "ORACLE_RNG_NOT_AVAILABLE",
            PositionOutOfBounds(_) => "ORACLE_POSITION_OUT_OF_BOUNDS",
            TileNotFound(_) => "ORACLE_TILE_NOT_FOUND",
            ItemNotFound(_) => "ORACLE_ITEM_NOT_FOUND",
            ActorTemplateNotFound(_) => "ORACLE_ACTOR_TEMPLATE_NOT_FOUND",
        }
    }
}

/// Aggregates read-only oracles required by the reducer and action pipeline.
#[derive(Clone, Copy, Debug)]
pub struct Env<'a, M, I, T, A, C, R>
where
    M: MapOracle + ?Sized,
    I: ItemOracle + ?Sized,
    T: TablesOracle + ?Sized,
    A: ActorOracle + ?Sized,
    C: ConfigOracle + ?Sized,
    R: RngOracle + ?Sized,
{
    map: Option<&'a M>,
    items: Option<&'a I>,
    tables: Option<&'a T>,
    actors: Option<&'a A>,
    config: Option<&'a C>,
    rng: Option<&'a R>,
}

pub type GameEnv<'a> = Env<
    'a,
    dyn MapOracle + 'a,
    dyn ItemOracle + 'a,
    dyn TablesOracle + 'a,
    dyn ActorOracle + 'a,
    dyn ConfigOracle + 'a,
    dyn RngOracle + 'a,
>;

impl<'a, M, I, T, A, C, R> Env<'a, M, I, T, A, C, R>
where
    M: MapOracle + ?Sized,
    I: ItemOracle + ?Sized,
    T: TablesOracle + ?Sized,
    A: ActorOracle + ?Sized,
    C: ConfigOracle + ?Sized,
    R: RngOracle + ?Sized,
{
    pub fn new(
        map: Option<&'a M>,
        items: Option<&'a I>,
        tables: Option<&'a T>,
        actors: Option<&'a A>,
        config: Option<&'a C>,
        rng: Option<&'a R>,
    ) -> Self {
        Self {
            map,
            items,
            tables,
            actors,
            config,
            rng,
        }
    }

    pub fn with_all(
        map: &'a M,
        items: &'a I,
        tables: &'a T,
        actors: &'a A,
        config: &'a C,
        rng: &'a R,
    ) -> Self {
        Self::new(
            Some(map),
            Some(items),
            Some(tables),
            Some(actors),
            Some(config),
            Some(rng),
        )
    }

    pub fn empty() -> Self {
        Self {
            map: None,
            items: None,
            tables: None,
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

    /// Returns the TablesOracle, or an error if not available.
    ///
    /// # Errors
    ///
    /// Returns `OracleError::TablesNotAvailable` if no tables oracle was provided.
    pub fn tables(&self) -> Result<&'a T, OracleError> {
        self.tables.ok_or(OracleError::TablesNotAvailable)
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
    T: TablesOracle + 'a,
    A: ActorOracle + 'a,
    C: ConfigOracle + 'a,
    R: RngOracle + 'a,
{
    pub fn into_game_env(self) -> GameEnv<'a> {
        let map: Option<&'a dyn MapOracle> = self.map.map(|map| map as _);
        let items: Option<&'a dyn ItemOracle> = self.items.map(|items| items as _);
        let tables: Option<&'a dyn TablesOracle> = self.tables.map(|tables| tables as _);
        let actors: Option<&'a dyn ActorOracle> = self.actors.map(|actors| actors as _);
        let config: Option<&'a dyn ConfigOracle> = self.config.map(|config| config as _);
        let rng: Option<&'a dyn RngOracle> = self.rng.map(|rng| rng as _);
        Env::new(map, items, tables, actors, config, rng)
    }
}
