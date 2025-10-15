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
use alloc::vec::Vec;

#[cfg(feature = "std")]
use std::vec::Vec;

use super::{
    AttackProfile, ConfigOracle, InitialEntitySpec, ItemDefinition, ItemOracle, MapDimensions,
    MapOracle, MovementRules, NpcOracle, NpcTemplate, StaticTile, TablesOracle,
};
use crate::{AttackStyle, GameConfig, ItemHandle, Position};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

// ============================================================================
// Snapshot Structures
// ============================================================================

/// Complete snapshot of all oracle data for zkVM guest execution
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct OracleSnapshot {
    pub map: MapSnapshot,
    pub items: ItemsSnapshot,
    pub npcs: NpcsSnapshot,
    pub tables: TablesSnapshot,
    pub config: ConfigSnapshot,
}

impl OracleSnapshot {
    pub fn new(
        map: MapSnapshot,
        items: ItemsSnapshot,
        npcs: NpcsSnapshot,
        tables: TablesSnapshot,
        config: ConfigSnapshot,
    ) -> Self {
        Self {
            map,
            items,
            npcs,
            tables,
            config,
        }
    }

    /// Creates a complete oracle snapshot from all oracle implementations.
    ///
    /// This is a convenience function for creating a snapshot from a bundle of oracles.
    #[cfg(feature = "std")]
    pub fn from_oracles(
        map: &impl MapOracle,
        _items: &impl ItemOracle,
        _npcs: &impl NpcOracle,
        tables: &impl TablesOracle,
        config: &impl ConfigOracle,
    ) -> Self {
        Self::new(
            MapSnapshot::from_oracle(map),
            ItemsSnapshot::empty(), // TODO: Need item handles list
            NpcsSnapshot::empty(),  // TODO: Need template IDs list
            TablesSnapshot::from_oracle(tables),
            ConfigSnapshot::from_oracle(config),
        )
    }
}

/// Snapshot of map oracle data
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct MapSnapshot {
    pub dimensions: MapDimensions,
    pub tiles: Vec<Option<StaticTile>>,
    pub initial_entities: Vec<InitialEntitySpec>,
}

impl MapSnapshot {
    pub fn new(
        dimensions: MapDimensions,
        tiles: Vec<Option<StaticTile>>,
        initial_entities: Vec<InitialEntitySpec>,
    ) -> Self {
        Self {
            dimensions,
            tiles,
            initial_entities,
        }
    }

    /// Creates a map snapshot from a MapOracle implementation.
    ///
    /// Traverses all tiles in the map and stores them in a flat row-major array.
    #[cfg(feature = "std")]
    pub fn from_oracle(oracle: &impl MapOracle) -> Self {
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

        let initial_entities = oracle.initial_entities();

        Self::new(dimensions, tiles, initial_entities)
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
    pub fn from_oracle(oracle: &impl ItemOracle, handles: &[ItemHandle]) -> Self {
        let items: Vec<(ItemHandle, ItemDefinition)> = handles
            .iter()
            .filter_map(|&handle| oracle.definition(handle).map(|def| (handle, def)))
            .collect();

        Self::new(items)
    }
}

/// Snapshot of NPCs oracle data
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct NpcsSnapshot {
    pub npcs: Vec<(u16, NpcTemplate)>,
}

impl NpcsSnapshot {
    pub fn new(npcs: Vec<(u16, NpcTemplate)>) -> Self {
        Self { npcs }
    }

    pub fn empty() -> Self {
        Self { npcs: Vec::new() }
    }

    /// Creates an NPCs snapshot from an NpcOracle.
    ///
    /// Note: This requires the list of all template IDs to query.
    #[cfg(feature = "std")]
    pub fn from_oracle(oracle: &impl NpcOracle, template_ids: &[u16]) -> Self {
        let npcs: Vec<(u16, NpcTemplate)> = template_ids
            .iter()
            .filter_map(|&id| oracle.template(id).map(|tmpl| (id, tmpl)))
            .collect();

        Self::new(npcs)
    }
}

/// Snapshot of tables oracle data
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct TablesSnapshot {
    pub movement_rules: MovementRules,
    pub attack_profiles: Vec<Option<AttackProfile>>,
}

impl TablesSnapshot {
    pub fn new(movement_rules: MovementRules, attack_profiles: Vec<Option<AttackProfile>>) -> Self {
        Self {
            movement_rules,
            attack_profiles,
        }
    }

    /// Creates a tables snapshot from a TablesOracle.
    #[cfg(feature = "std")]
    pub fn from_oracle(oracle: &impl TablesOracle) -> Self {
        let movement_rules = oracle.movement_rules();

        // Query all attack styles to build the profiles array
        let attack_profiles = vec![oracle.attack_profile(AttackStyle::Melee)];

        Self::new(movement_rules, attack_profiles)
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
    pub fn from_oracle(oracle: &impl ConfigOracle) -> Self {
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

    fn initial_entities(&self) -> Vec<InitialEntitySpec> {
        self.snapshot.initial_entities.clone()
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

/// Guest-side adapter for NpcOracle backed by NpcsSnapshot
pub struct SnapshotNpcOracle<'a> {
    snapshot: &'a NpcsSnapshot,
}

impl<'a> SnapshotNpcOracle<'a> {
    pub fn new(snapshot: &'a NpcsSnapshot) -> Self {
        Self { snapshot }
    }
}

impl<'a> NpcOracle for SnapshotNpcOracle<'a> {
    fn template(&self, template_id: u16) -> Option<NpcTemplate> {
        self.snapshot
            .npcs
            .iter()
            .find(|(id, _)| *id == template_id)
            .map(|(_, tmpl)| tmpl.clone())
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
    fn movement_rules(&self) -> MovementRules {
        self.snapshot.movement_rules
    }

    fn attack_profile(&self, style: AttackStyle) -> Option<AttackProfile> {
        let index = style as usize;
        self.snapshot
            .attack_profiles
            .get(index)
            .and_then(|profile| *profile)
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
    pub npcs: SnapshotNpcOracle<'a>,
    pub config: SnapshotConfigOracle<'a>,
}

impl<'a> SnapshotOracleBundle<'a> {
    /// Creates an oracle bundle from a snapshot
    pub fn new(snapshot: &'a OracleSnapshot) -> Self {
        Self {
            map: SnapshotMapOracle::new(&snapshot.map),
            items: SnapshotItemOracle::new(&snapshot.items),
            tables: SnapshotTablesOracle::new(&snapshot.tables),
            npcs: SnapshotNpcOracle::new(&snapshot.npcs),
            config: SnapshotConfigOracle::new(&snapshot.config),
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
        SnapshotNpcOracle<'a>,
        SnapshotConfigOracle<'a>,
    > {
        super::Env::with_all(
            &self.map,
            &self.items,
            &self.tables,
            &self.npcs,
            &self.config,
        )
    }
}
