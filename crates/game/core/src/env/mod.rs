//! Traits describing read-only world data.
//!
//! Oracles expose static map geometry, item definitions, rule tables, and NPC
//! templates. The [`Env`] aggregate bundles them so the engine can access
//! everything it needs without hard coupling to concrete implementations.
mod config;
mod items;
mod map;
mod npc;
mod snapshot;
mod tables;

pub use config::ConfigOracle;
pub use items::{
    ArmorData, ConsumableData, ConsumableEffect, ItemDefinition, ItemKind, ItemOracle, WeaponData,
};
pub use map::{MapDimensions, MapOracle, StaticTile, TerrainKind};
pub use npc::{ActorOracle, ActorTemplate, ActorTemplateBuilder};
pub use snapshot::{
    ActorsSnapshot, ConfigSnapshot, ItemsSnapshot, MapSnapshot, OracleSnapshot,
    SnapshotActorOracle, SnapshotConfigOracle, SnapshotItemOracle, SnapshotMapOracle,
    SnapshotOracleBundle, SnapshotTablesOracle, TablesSnapshot,
};
pub use tables::TablesOracle;

/// Aggregates read-only oracles required by the reducer and action pipeline.
#[derive(Clone, Copy, Debug)]
pub struct Env<'a, M, I, T, A, C>
where
    M: MapOracle + ?Sized,
    I: ItemOracle + ?Sized,
    T: TablesOracle + ?Sized,
    A: ActorOracle + ?Sized,
    C: ConfigOracle + ?Sized,
{
    map: Option<&'a M>,
    items: Option<&'a I>,
    tables: Option<&'a T>,
    actors: Option<&'a A>,
    config: Option<&'a C>,
}

pub type GameEnv<'a> = Env<
    'a,
    dyn MapOracle + 'a,
    dyn ItemOracle + 'a,
    dyn TablesOracle + 'a,
    dyn ActorOracle + 'a,
    dyn ConfigOracle + 'a,
>;

impl<'a, M, I, T, A, C> Env<'a, M, I, T, A, C>
where
    M: MapOracle + ?Sized,
    I: ItemOracle + ?Sized,
    T: TablesOracle + ?Sized,
    A: ActorOracle + ?Sized,
    C: ConfigOracle + ?Sized,
{
    pub fn new(
        map: Option<&'a M>,
        items: Option<&'a I>,
        tables: Option<&'a T>,
        actors: Option<&'a A>,
        config: Option<&'a C>,
    ) -> Self {
        Self {
            map,
            items,
            tables,
            actors,
            config,
        }
    }

    pub fn with_all(map: &'a M, items: &'a I, tables: &'a T, actors: &'a A, config: &'a C) -> Self {
        Self::new(
            Some(map),
            Some(items),
            Some(tables),
            Some(actors),
            Some(config),
        )
    }

    pub fn empty() -> Self {
        Self {
            map: None,
            items: None,
            tables: None,
            actors: None,
            config: None,
        }
    }

    pub fn map(&self) -> Option<&'a M> {
        self.map
    }

    pub fn items(&self) -> Option<&'a I> {
        self.items
    }

    pub fn tables(&self) -> Option<&'a T> {
        self.tables
    }

    pub fn actors(&self) -> Option<&'a A> {
        self.actors
    }

    pub fn config(&self) -> Option<&'a C> {
        self.config
    }

    /// Returns the activation radius from the config oracle.
    /// Defaults to 0 if no config oracle is provided.
    pub fn activation_radius(&self) -> u32 {
        self.config.map(|c| c.activation_radius()).unwrap_or(0)
    }
}

impl<'a, M, I, T, A, C> Env<'a, M, I, T, A, C>
where
    M: MapOracle + 'a,
    I: ItemOracle + 'a,
    T: TablesOracle + 'a,
    A: ActorOracle + 'a,
    C: ConfigOracle + 'a,
{
    pub fn into_game_env(self) -> GameEnv<'a> {
        let map: Option<&'a dyn MapOracle> = self.map.map(|map| map as _);
        let items: Option<&'a dyn ItemOracle> = self.items.map(|items| items as _);
        let tables: Option<&'a dyn TablesOracle> = self.tables.map(|tables| tables as _);
        let actors: Option<&'a dyn ActorOracle> = self.actors.map(|actors| actors as _);
        let config: Option<&'a dyn ConfigOracle> = self.config.map(|config| config as _);
        Env::new(map, items, tables, actors, config)
    }
}
