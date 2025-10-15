//! Oracle snapshot for serializable game content.
//!
//! This module provides serializable snapshots of all oracle data required
//! by the guest program to execute game logic deterministically.
//!
//! # Design Pattern: Snapshot Pattern
//!
//! The OracleSnapshot captures immutable oracle state at a point in time,
//! enabling deterministic replay in the zkVM guest environment.

use game_core::{
    AttackProfile, AttackStyle, GameConfig, InitialEntitySpec, ItemDefinition, ItemHandle,
    MapDimensions, MovementRules, NpcTemplate, Position, StaticTile,
};
use serde::{Deserialize, Serialize};

/// Complete snapshot of all oracle data for guest program execution.
///
/// This structure captures all static game content needed by the guest
/// to execute actions deterministically. It is serialized and sent to
/// the zkVM guest program via ExecutorEnv.
///
/// # Invariants
///
/// - All data is immutable after creation
/// - Snapshot represents a consistent view of oracles at a single point in time
/// - All lookups use simple linear search (guest environment optimization)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OracleSnapshot {
    pub map: MapSnapshot,
    pub items: ItemsSnapshot,
    pub npcs: NpcsSnapshot,
    pub tables: TablesSnapshot,
    pub config: ConfigSnapshot,
}

impl OracleSnapshot {
    /// Creates a new oracle snapshot with all components.
    ///
    /// Use [`OracleSnapshotBuilder`] for more ergonomic construction.
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

    /// Creates a builder for constructing oracle snapshots.
    pub fn builder() -> OracleSnapshotBuilder {
        OracleSnapshotBuilder::default()
    }
}

// ============================================================================
// Conversion from Runtime Oracles
// ============================================================================

impl MapSnapshot {
    /// Creates a map snapshot from a MapOracle implementation.
    ///
    /// This function traverses all tiles in the map and stores them in a
    /// flat row-major array for efficient serialization.
    pub fn from_oracle(oracle: &impl game_core::MapOracle) -> Self {
        let dimensions = oracle.dimensions();
        let capacity = (dimensions.width * dimensions.height) as usize;
        let mut tiles = Vec::with_capacity(capacity);

        // Traverse all positions in row-major order
        for y in 0..dimensions.height as i32 {
            for x in 0..dimensions.width as i32 {
                let position = Position { x, y };
                let tile = oracle.tile(position);
                tiles.push(tile);
            }
        }

        let initial_entities = oracle.initial_entities();

        Self::new(dimensions, tiles, initial_entities)
    }
}

impl ItemsSnapshot {
    /// Creates an items snapshot from an ItemOracle implementation.
    ///
    /// **Note**: This requires the oracle to expose all item handles.
    /// Since the current ItemOracle trait doesn't provide iteration,
    /// this function requires a separate `handles` parameter.
    ///
    /// # Arguments
    ///
    /// * `oracle` - The item oracle implementation
    /// * `handles` - All item handles to snapshot
    pub fn from_oracle(oracle: &impl game_core::ItemOracle, handles: &[ItemHandle]) -> Self {
        let items: Vec<(ItemHandle, ItemDefinition)> = handles
            .iter()
            .filter_map(|&handle| oracle.definition(handle).map(|def| (handle, def.clone())))
            .collect();

        Self::new(items)
    }

    /// Creates an empty items snapshot (for testing or when no items exist).
    pub fn empty() -> Self {
        Self::new(vec![])
    }
}

impl NpcsSnapshot {
    /// Creates an NPCs snapshot from an NpcOracle implementation.
    ///
    /// **Note**: Similar to ItemsSnapshot, this requires explicit template IDs
    /// since the NpcOracle trait doesn't provide iteration.
    ///
    /// # Arguments
    ///
    /// * `oracle` - The NPC oracle implementation
    /// * `template_ids` - All NPC template IDs to snapshot
    pub fn from_oracle(oracle: &impl game_core::NpcOracle, template_ids: &[u16]) -> Self {
        let npcs: Vec<(u16, NpcTemplate)> = template_ids
            .iter()
            .filter_map(|&id| oracle.template(id).map(|tmpl| (id, tmpl)))
            .collect();

        Self::new(npcs)
    }

    /// Creates an empty NPCs snapshot (for testing or when no NPCs exist).
    pub fn empty() -> Self {
        Self::new(vec![])
    }
}

impl TablesSnapshot {
    /// Creates a tables snapshot from a TablesOracle implementation.
    ///
    /// This function queries attack profiles for all known AttackStyle variants.
    pub fn from_oracle(oracle: &impl game_core::TablesOracle) -> Self {
        use game_core::AttackStyle;

        let movement_rules = oracle.movement_rules();

        // Query attack profiles for all attack styles
        // Currently only Melee is defined
        let attack_styles = [AttackStyle::Melee];

        let mut attack_profiles = vec![None; attack_styles.len()];
        for (idx, style) in attack_styles.iter().enumerate() {
            attack_profiles[idx] = oracle.attack_profile(style.clone());
        }

        Self::new(movement_rules, attack_profiles)
    }
}

impl ConfigSnapshot {
    /// Creates a config snapshot from a ConfigOracle implementation.
    pub fn from_oracle(oracle: &impl game_core::ConfigOracle) -> Self {
        Self::new(oracle.activation_radius())
    }
}

// ============================================================================
// Map Oracle Snapshot
// ============================================================================

/// Snapshot of map data with spatial tile lookups.
///
/// # Storage Strategy
///
/// Tiles are stored in a flat Vec in row-major order for efficient
/// serialization and cache locality. Position-based lookups compute
/// the index: `y * width + x`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapSnapshot {
    pub dimensions: MapDimensions,
    /// Tiles in row-major order (index = y * width + x)
    pub tiles: Vec<Option<StaticTile>>,
    pub initial_entities: Vec<InitialEntitySpec>,
}

impl MapSnapshot {
    /// Creates a new map snapshot.
    ///
    /// # Arguments
    ///
    /// * `dimensions` - Map dimensions
    /// * `tiles` - Flat array of tiles in row-major order
    /// * `initial_entities` - Entity spawn specifications
    pub fn new(
        dimensions: MapDimensions,
        tiles: Vec<Option<StaticTile>>,
        initial_entities: Vec<InitialEntitySpec>,
    ) -> Self {
        debug_assert_eq!(
            tiles.len(),
            (dimensions.width * dimensions.height) as usize,
            "Tile array size must match dimensions"
        );

        Self {
            dimensions,
            tiles,
            initial_entities,
        }
    }

    /// Gets the tile at the given position.
    ///
    /// Returns None if position is out of bounds.
    pub fn get_tile(&self, position: Position) -> Option<&StaticTile> {
        if !self.dimensions.contains(position) {
            return None;
        }

        let index = self.position_to_index(position);
        self.tiles.get(index).and_then(|tile| tile.as_ref())
    }

    /// Converts a position to a flat array index.
    #[inline]
    fn position_to_index(&self, position: Position) -> usize {
        (position.y * self.dimensions.width as i32 + position.x) as usize
    }
}

// ============================================================================
// Items Oracle Snapshot
// ============================================================================

/// Snapshot of item definitions.
///
/// # Storage Strategy
///
/// Uses a simple Vec of (handle, definition) pairs. Guest environment
/// performs linear search since item count is typically small (<100).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemsSnapshot {
    /// Item definitions indexed by handle
    pub items: Vec<(ItemHandle, ItemDefinition)>,
}

impl ItemsSnapshot {
    /// Creates a new items snapshot.
    pub fn new(items: Vec<(ItemHandle, ItemDefinition)>) -> Self {
        Self { items }
    }

    /// Gets the definition for a given item handle.
    ///
    /// Performs linear search. Acceptable for small item counts.
    pub fn definition(&self, handle: ItemHandle) -> Option<&ItemDefinition> {
        self.items
            .iter()
            .find(|(h, _)| *h == handle)
            .map(|(_, def)| def)
    }

    /// Returns the number of item definitions.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Returns true if there are no item definitions.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

// ============================================================================
// NPCs Oracle Snapshot
// ============================================================================

/// Snapshot of NPC templates.
///
/// # Storage Strategy
///
/// Similar to items, uses Vec for simple linear search in guest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NpcsSnapshot {
    /// NPC templates indexed by template ID
    pub npcs: Vec<(u16, NpcTemplate)>,
}

impl NpcsSnapshot {
    /// Creates a new NPCs snapshot.
    pub fn new(npcs: Vec<(u16, NpcTemplate)>) -> Self {
        Self { npcs }
    }

    /// Gets the template for a given template ID.
    ///
    /// Performs linear search. Acceptable for small NPC template counts.
    pub fn template(&self, template_id: u16) -> Option<&NpcTemplate> {
        self.npcs
            .iter()
            .find(|(id, _)| *id == template_id)
            .map(|(_, tmpl)| tmpl)
    }

    /// Returns the number of NPC templates.
    pub fn len(&self) -> usize {
        self.npcs.len()
    }

    /// Returns true if there are no NPC templates.
    pub fn is_empty(&self) -> bool {
        self.npcs.is_empty()
    }
}

// ============================================================================
// Tables Oracle Snapshot
// ============================================================================

/// Snapshot of game rules and balance tables.
///
/// # Storage Strategy
///
/// Attack profiles are stored in a Vec indexed by AttackStyle discriminant.
/// This provides O(1) lookup in both host and guest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TablesSnapshot {
    pub movement_rules: MovementRules,
    /// Attack profiles indexed by AttackStyle
    /// Index corresponds to AttackStyle as u8
    pub attack_profiles: Vec<Option<AttackProfile>>,
}

impl TablesSnapshot {
    /// Creates a new tables snapshot.
    ///
    /// Attack profiles vec should have entries for each AttackStyle variant.
    pub fn new(movement_rules: MovementRules, attack_profiles: Vec<Option<AttackProfile>>) -> Self {
        Self {
            movement_rules,
            attack_profiles,
        }
    }

    /// Gets the attack profile for a given attack style.
    pub fn attack_profile(&self, style: AttackStyle) -> Option<&AttackProfile> {
        let index = style as usize;
        self.attack_profiles
            .get(index)
            .and_then(|profile| profile.as_ref())
    }
}

// ============================================================================
// Config Oracle Snapshot
// ============================================================================

/// Snapshot of runtime configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigSnapshot {
    pub activation_radius: u32,
}

impl ConfigSnapshot {
    /// Creates a new config snapshot.
    pub fn new(activation_radius: u32) -> Self {
        Self { activation_radius }
    }

    /// Creates a config snapshot from GameConfig.
    pub fn from_game_config(config: &GameConfig) -> Self {
        Self {
            activation_radius: config.activation_radius,
        }
    }
}

// ============================================================================
// Builder Pattern for OracleSnapshot
// ============================================================================

/// Builder for constructing OracleSnapshot instances.
///
/// # Example
///
/// ```ignore
/// let snapshot = OracleSnapshot::builder()
///     .map(map_snapshot)
///     .items(items_snapshot)
///     .npcs(npcs_snapshot)
///     .tables(tables_snapshot)
///     .config(config_snapshot)
///     .build();
/// ```
#[derive(Default)]
pub struct OracleSnapshotBuilder {
    map: Option<MapSnapshot>,
    items: Option<ItemsSnapshot>,
    npcs: Option<NpcsSnapshot>,
    tables: Option<TablesSnapshot>,
    config: Option<ConfigSnapshot>,
}

impl OracleSnapshotBuilder {
    /// Sets the map snapshot.
    pub fn map(mut self, map: MapSnapshot) -> Self {
        self.map = Some(map);
        self
    }

    /// Sets the items snapshot.
    pub fn items(mut self, items: ItemsSnapshot) -> Self {
        self.items = Some(items);
        self
    }

    /// Sets the NPCs snapshot.
    pub fn npcs(mut self, npcs: NpcsSnapshot) -> Self {
        self.npcs = Some(npcs);
        self
    }

    /// Sets the tables snapshot.
    pub fn tables(mut self, tables: TablesSnapshot) -> Self {
        self.tables = Some(tables);
        self
    }

    /// Sets the config snapshot.
    pub fn config(mut self, config: ConfigSnapshot) -> Self {
        self.config = Some(config);
        self
    }

    /// Builds the OracleSnapshot.
    ///
    /// # Panics
    ///
    /// Panics if any required component is missing.
    pub fn build(self) -> OracleSnapshot {
        OracleSnapshot::new(
            self.map.expect("map snapshot required"),
            self.items.expect("items snapshot required"),
            self.npcs.expect("npcs snapshot required"),
            self.tables.expect("tables snapshot required"),
            self.config.expect("config snapshot required"),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use game_core::{EntityId, InitialEntityKind, InitialEntitySpec, ItemCategory, TerrainKind};

    // Mock oracle implementations for testing
    struct MockMapOracle {
        dimensions: MapDimensions,
        tiles: Vec<Option<StaticTile>>,
        entities: Vec<InitialEntitySpec>,
    }

    impl game_core::MapOracle for MockMapOracle {
        fn dimensions(&self) -> MapDimensions {
            self.dimensions
        }

        fn tile(&self, position: Position) -> Option<StaticTile> {
            let idx = (position.y * self.dimensions.width as i32 + position.x) as usize;
            self.tiles.get(idx).and_then(|t| *t)
        }

        fn initial_entities(&self) -> Vec<InitialEntitySpec> {
            self.entities.clone()
        }
    }

    #[test]
    fn test_map_snapshot_from_oracle() {
        let mock_oracle = MockMapOracle {
            dimensions: MapDimensions::new(2, 2),
            tiles: vec![
                Some(StaticTile::new(TerrainKind::Floor)),
                Some(StaticTile::new(TerrainKind::Wall)),
                Some(StaticTile::new(TerrainKind::Floor)),
                Some(StaticTile::new(TerrainKind::Floor)),
            ],
            entities: vec![InitialEntitySpec {
                id: EntityId::PLAYER,
                position: Position::new(0, 0),
                kind: InitialEntityKind::Player,
            }],
        };

        let snapshot = MapSnapshot::from_oracle(&mock_oracle);

        assert_eq!(snapshot.dimensions, MapDimensions::new(2, 2));
        assert_eq!(snapshot.tiles.len(), 4);
        assert_eq!(
            snapshot.get_tile(Position::new(0, 0)),
            Some(&StaticTile::new(TerrainKind::Floor))
        );
        assert_eq!(
            snapshot.get_tile(Position::new(1, 0)),
            Some(&StaticTile::new(TerrainKind::Wall))
        );
        assert_eq!(snapshot.initial_entities.len(), 1);
    }

    #[test]
    fn test_map_snapshot_position_to_index() {
        let dimensions = MapDimensions::new(10, 10);
        let tiles = vec![None; 100];
        let snapshot = MapSnapshot::new(dimensions, tiles, vec![]);

        // Test corner cases
        assert_eq!(snapshot.position_to_index(Position::new(0, 0)), 0);
        assert_eq!(snapshot.position_to_index(Position::new(9, 0)), 9);
        assert_eq!(snapshot.position_to_index(Position::new(0, 9)), 90);
        assert_eq!(snapshot.position_to_index(Position::new(9, 9)), 99);

        // Test middle
        assert_eq!(snapshot.position_to_index(Position::new(5, 5)), 55);
    }

    #[test]
    fn test_items_snapshot_lookup() {
        let items = vec![
            (
                ItemHandle(1),
                ItemDefinition::new(ItemHandle(1), ItemCategory::Consumable, None, None),
            ),
            (
                ItemHandle(42),
                ItemDefinition::new(ItemHandle(42), ItemCategory::Key, None, None),
            ),
        ];

        let snapshot = ItemsSnapshot::new(items);

        assert!(snapshot.definition(ItemHandle(1)).is_some());
        assert!(snapshot.definition(ItemHandle(42)).is_some());
        assert!(snapshot.definition(ItemHandle(999)).is_none());
    }

    #[test]
    fn test_builder_pattern() {
        let map = MapSnapshot::new(MapDimensions::new(10, 10), vec![None; 100], vec![]);
        let items = ItemsSnapshot::new(vec![]);
        let npcs = NpcsSnapshot::new(vec![]);
        let tables = TablesSnapshot::new(MovementRules::new(1, 100), vec![]);
        let config = ConfigSnapshot::new(5);

        let snapshot = OracleSnapshot::builder()
            .map(map.clone())
            .items(items.clone())
            .npcs(npcs.clone())
            .tables(tables.clone())
            .config(config.clone())
            .build();

        assert_eq!(snapshot.map, map);
        assert_eq!(snapshot.items, items);
        assert_eq!(snapshot.npcs, npcs);
        assert_eq!(snapshot.tables, tables);
        assert_eq!(snapshot.config, config);
    }

    #[test]
    fn test_serialization_roundtrip() {
        let map = MapSnapshot::new(
            MapDimensions::new(2, 2),
            vec![
                Some(StaticTile::new(TerrainKind::Floor)),
                Some(StaticTile::new(TerrainKind::Wall)),
                Some(StaticTile::new(TerrainKind::Floor)),
                Some(StaticTile::new(TerrainKind::Floor)),
            ],
            vec![],
        );

        let items = ItemsSnapshot::new(vec![(
            ItemHandle(1),
            ItemDefinition::new(ItemHandle(1), ItemCategory::Consumable, Some(5), Some(3)),
        )]);

        let npcs = NpcsSnapshot::new(vec![]);
        let tables = TablesSnapshot::new(MovementRules::new(1, 100), vec![]);
        let config = ConfigSnapshot::new(5);

        let snapshot = OracleSnapshot::new(map, items, npcs, tables, config);

        // Serialize
        let bytes = bincode::serialize(&snapshot).expect("serialization failed");

        // Deserialize
        let deserialized: OracleSnapshot =
            bincode::deserialize(&bytes).expect("deserialization failed");

        assert_eq!(snapshot, deserialized);
    }
}
