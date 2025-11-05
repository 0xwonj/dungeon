//! Oracle snapshots and guest adapters for zkVM execution.
//!
//! This module provides serializable snapshots of oracle data and
//! adapter implementations that work in `no_std` environments like zkVM guests.
//!
//! # Design
//!
//! - **Snapshots**: Serializable structures that capture oracle state
//! - **Guest Adapters**: Implement oracle traits backed by snapshots
//! - **no_std**: Works in both std and no_std environments

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(not(feature = "std"))]
use alloc::{collections::BTreeMap, vec::Vec};

#[cfg(feature = "std")]
use std::{collections::BTreeMap, vec::Vec};

use super::{
    ActorOracle, ConfigOracle, ItemDefinition, ItemOracle, MapDimensions, MapOracle, StaticTile,
    TablesOracle,
};
use crate::{GameConfig, ItemHandle, Position};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

// ============================================================================
// Snapshot Structures
// ============================================================================

/// Complete snapshot of all oracle data for zkVM guest execution
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct OracleSnapshot {
    pub map: MapSnapshot,
    pub items: ItemsSnapshot,
    pub actors: ActorsSnapshot,
    pub tables: TablesSnapshot,
    pub config: ConfigSnapshot,
}

impl OracleSnapshot {
    pub fn new(
        map: MapSnapshot,
        items: ItemsSnapshot,
        actors: ActorsSnapshot,
        tables: TablesSnapshot,
        config: ConfigSnapshot,
    ) -> Self {
        Self {
            map,
            items,
            actors,
            tables,
            config,
        }
    }

    /// Creates a complete oracle snapshot from all oracle implementations.
    ///
    /// # Arguments
    ///
    /// * `actor_ids` - List of actor definition IDs to include in snapshot
    ///   (should come from scenario or other source)
    #[cfg(feature = "std")]
    pub fn from_oracles(
        map: &dyn MapOracle,
        _items: &dyn ItemOracle,
        actors: &dyn ActorOracle,
        tables: &dyn TablesOracle,
        config: &dyn ConfigOracle,
        actor_ids: &[String],
    ) -> Self {
        Self::new(
            MapSnapshot::from_oracle(map),
            ItemsSnapshot::empty(), // TODO: Need item handles from scenario
            ActorsSnapshot::from_oracle(actors, actor_ids),
            TablesSnapshot::from_oracle(tables),
            ConfigSnapshot::from_oracle(config),
        )
    }
}

/// Snapshot of map oracle data (terrain only, no entities)
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct MapSnapshot {
    pub dimensions: MapDimensions,
    pub tiles: Vec<Option<StaticTile>>,
}

impl MapSnapshot {
    pub fn new(dimensions: MapDimensions, tiles: Vec<Option<StaticTile>>) -> Self {
        Self { dimensions, tiles }
    }

    /// Creates a map snapshot from a MapOracle implementation.
    ///
    /// Traverses all tiles in the map and stores them in a flat row-major array.
    #[cfg(feature = "std")]
    pub fn from_oracle(oracle: &dyn MapOracle) -> Self {
        let dimensions = oracle.dimensions();
        let capacity = (dimensions.width * dimensions.height) as usize;
        let mut tiles = Vec::with_capacity(capacity);

        // Traverse all positions in row-major order
        for y in 0..dimensions.height as i32 {
            for x in 0..dimensions.width as i32 {
                let pos = Position { x, y };
                tiles.push(oracle.tile(pos));
            }
        }

        Self::new(dimensions, tiles)
    }
}

/// Snapshot of items oracle data
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ItemsSnapshot {
    pub items: Vec<(ItemHandle, ItemDefinition)>,
}

impl ItemsSnapshot {
    pub fn new(items: Vec<(ItemHandle, ItemDefinition)>) -> Self {
        Self { items }
    }

    pub fn empty() -> Self {
        Self { items: Vec::new() }
    }

    /// Creates an items snapshot from an ItemOracle.
    ///
    /// Note: This requires the list of all item handles to query.
    #[cfg(feature = "std")]
    pub fn from_oracle(oracle: &dyn ItemOracle, handles: &[ItemHandle]) -> Self {
        let items: Vec<(ItemHandle, ItemDefinition)> = handles
            .iter()
            .filter_map(|&handle| oracle.definition(handle).map(|def| (handle, def)))
            .collect();

        Self::new(items)
    }
}

// Type alias for String based on std/no_std
#[cfg(not(feature = "std"))]
type ActorId = alloc::string::String;
#[cfg(feature = "std")]
type ActorId = String;

/// Snapshot of actors oracle data containing actor templates by definition ID.
///
/// This snapshot stores all actor templates that need to be available in the zkVM guest.
/// Templates are stored by their definition ID (e.g., "player", "goblin_scout").
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ActorsSnapshot {
    /// Vector of (definition_id, template) pairs.
    pub templates: Vec<(ActorId, super::ActorTemplate)>,
}

impl ActorsSnapshot {
    /// Create a new actors snapshot with the given templates.
    pub fn new(templates: Vec<(ActorId, super::ActorTemplate)>) -> Self {
        Self { templates }
    }

    /// Create an empty snapshot with no templates.
    pub fn empty() -> Self {
        Self {
            templates: Vec::new(),
        }
    }

    /// Creates an actors snapshot from an ActorOracle implementation.
    ///
    /// # Arguments
    ///
    /// * `oracle` - Actor oracle implementation
    /// * `def_ids` - List of actor definition IDs to snapshot
    #[cfg(feature = "std")]
    pub fn from_oracle(oracle: &dyn super::ActorOracle, def_ids: &[String]) -> Self {
        let mut templates = Vec::with_capacity(def_ids.len());

        for id in def_ids {
            if let Some(template) = oracle.template(id) {
                templates.push((id.clone(), template));
            }
        }

        Self::new(templates)
    }
}

/// Snapshot of tables oracle data
///
/// This snapshot captures all game balance values for deterministic
/// execution in zkVM and future on-chain verification.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct TablesSnapshot {
    pub action_costs: super::ActionCosts,
    pub combat: super::CombatParams,
    pub speed: super::SpeedParams,
    pub action_profiles: BTreeMap<crate::action::ActionKind, crate::action::ActionProfile>,
}

impl TablesSnapshot {
    pub fn new(
        action_costs: super::ActionCosts,
        combat: super::CombatParams,
        speed: super::SpeedParams,
        action_profiles: BTreeMap<crate::action::ActionKind, crate::action::ActionProfile>,
    ) -> Self {
        Self {
            action_costs,
            combat,
            speed,
            action_profiles,
        }
    }

    /// Creates a tables snapshot from a TablesOracle.
    #[cfg(feature = "std")]
    pub fn from_oracle(oracle: &dyn TablesOracle) -> Self {
        // Load all action profiles from the oracle
        let mut action_profiles = BTreeMap::new();
        for &kind in crate::action::ActionKind::all_variants() {
            let profile = oracle.action_profile(kind);
            action_profiles.insert(kind, profile);
        }

        Self::new(
            oracle.action_costs(),
            oracle.combat(),
            oracle.speed(),
            action_profiles,
        )
    }
}

/// Snapshot of config oracle data
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ConfigSnapshot {
    pub config: GameConfig,
}

impl ConfigSnapshot {
    pub fn new(config: GameConfig) -> Self {
        Self { config }
    }

    /// Creates a config snapshot from a ConfigOracle.
    #[cfg(feature = "std")]
    pub fn from_oracle(oracle: &dyn ConfigOracle) -> Self {
        let config = GameConfig {
            activation_radius: oracle.activation_radius(),
        };
        Self::new(config)
    }
}

// ============================================================================
// Guest Adapters (implement oracle traits backed by snapshots)
// ============================================================================

/// Guest-side adapter for MapOracle backed by MapSnapshot
pub struct SnapshotMapOracle<'a> {
    snapshot: &'a MapSnapshot,
}

impl<'a> SnapshotMapOracle<'a> {
    pub fn new(snapshot: &'a MapSnapshot) -> Self {
        Self { snapshot }
    }
}

impl<'a> MapOracle for SnapshotMapOracle<'a> {
    fn dimensions(&self) -> MapDimensions {
        self.snapshot.dimensions
    }

    fn tile(&self, pos: Position) -> Option<StaticTile> {
        let dims = self.snapshot.dimensions;
        if pos.x < 0 || pos.y < 0 || pos.x >= dims.width as i32 || pos.y >= dims.height as i32 {
            return None;
        }
        let index = (pos.y as usize * dims.width as usize) + pos.x as usize;
        self.snapshot.tiles.get(index).and_then(|t| *t)
    }
}

/// Guest-side adapter for ItemOracle backed by ItemsSnapshot
pub struct SnapshotItemOracle<'a> {
    snapshot: &'a ItemsSnapshot,
}

impl<'a> SnapshotItemOracle<'a> {
    pub fn new(snapshot: &'a ItemsSnapshot) -> Self {
        Self { snapshot }
    }
}

impl<'a> ItemOracle for SnapshotItemOracle<'a> {
    fn definition(&self, handle: ItemHandle) -> Option<ItemDefinition> {
        self.snapshot
            .items
            .iter()
            .find(|(h, _)| *h == handle)
            .map(|(_, def)| def.clone())
    }
}

/// Guest-side adapter for ActorOracle backed by ActorsSnapshot
pub struct SnapshotActorOracle<'a> {
    snapshot: &'a ActorsSnapshot,
}

impl<'a> SnapshotActorOracle<'a> {
    pub fn new(snapshot: &'a ActorsSnapshot) -> Self {
        Self { snapshot }
    }
}

impl<'a> ActorOracle for SnapshotActorOracle<'a> {
    fn template(&self, def_id: &str) -> Option<super::ActorTemplate> {
        self.snapshot
            .templates
            .iter()
            .find(|(id, _)| id.as_str() == def_id)
            .map(|(_, template)| template.clone())
    }
}

/// Guest-side adapter for TablesOracle backed by TablesSnapshot
pub struct SnapshotTablesOracle<'a> {
    snapshot: &'a TablesSnapshot,
}

impl<'a> SnapshotTablesOracle<'a> {
    pub fn new(snapshot: &'a TablesSnapshot) -> Self {
        Self { snapshot }
    }
}

impl<'a> TablesOracle for SnapshotTablesOracle<'a> {
    fn action_costs(&self) -> super::ActionCosts {
        self.snapshot.action_costs
    }

    fn combat(&self) -> super::CombatParams {
        self.snapshot.combat
    }

    fn speed(&self) -> super::SpeedParams {
        self.snapshot.speed
    }

    fn action_profile(&self, kind: crate::action::ActionKind) -> crate::action::ActionProfile {
        self.snapshot
            .action_profiles
            .get(&kind)
            .cloned()
            .unwrap_or_else(|| {
                panic!(
                    "ActionProfile for {:?} not found in snapshot. \
                     This action may not have RON data defined.",
                    kind
                )
            })
    }
}

/// Guest-side adapter for ConfigOracle backed by ConfigSnapshot
pub struct SnapshotConfigOracle<'a> {
    snapshot: &'a ConfigSnapshot,
}

impl<'a> SnapshotConfigOracle<'a> {
    pub fn new(snapshot: &'a ConfigSnapshot) -> Self {
        Self { snapshot }
    }
}

impl<'a> ConfigOracle for SnapshotConfigOracle<'a> {
    fn activation_radius(&self) -> u32 {
        self.snapshot.config.activation_radius
    }
}

/// Bundle of all snapshot-backed oracle adapters.
///
/// This owns all adapters to avoid lifetime issues in guest programs.
pub struct SnapshotOracleBundle<'a> {
    pub map: SnapshotMapOracle<'a>,
    pub items: SnapshotItemOracle<'a>,
    pub tables: SnapshotTablesOracle<'a>,
    pub actors: SnapshotActorOracle<'a>,
    pub config: SnapshotConfigOracle<'a>,
    pub rng: super::PcgRng,
}

impl<'a> SnapshotOracleBundle<'a> {
    /// Creates an oracle bundle from a snapshot
    pub fn new(snapshot: &'a OracleSnapshot) -> Self {
        Self {
            map: SnapshotMapOracle::new(&snapshot.map),
            items: SnapshotItemOracle::new(&snapshot.items),
            tables: SnapshotTablesOracle::new(&snapshot.tables),
            actors: SnapshotActorOracle::new(&snapshot.actors),
            config: SnapshotConfigOracle::new(&snapshot.config),
            rng: super::PcgRng, // PcgRng is stateless
        }
    }

    /// Creates a game Env from this bundle
    pub fn as_env(
        &self,
    ) -> super::Env<
        '_,
        SnapshotMapOracle<'a>,
        SnapshotItemOracle<'a>,
        SnapshotTablesOracle<'a>,
        SnapshotActorOracle<'a>,
        SnapshotConfigOracle<'a>,
        super::PcgRng,
    > {
        super::Env::with_all(
            &self.map,
            &self.items,
            &self.tables,
            &self.actors,
            &self.config,
            &self.rng,
        )
    }
}
