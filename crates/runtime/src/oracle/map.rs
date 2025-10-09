//! Static dungeon layout served through [`game_core::MapOracle`].
use game_core::{
    EntityId, InitialEntityKind, InitialEntitySpec, MapDimensions, MapOracle, Position, StaticTile,
    TerrainKind,
};
use std::collections::HashMap;

/// MapOracle implementation with static map data
///
/// Holds immutable map structure that doesn't change during gameplay.
/// For dynamic map changes (doors opening, etc.), that would go in GameState.
pub struct MapOracleImpl {
    dimensions: MapDimensions,
    tiles: HashMap<Position, StaticTile>,
    initial_entities: Vec<InitialEntitySpec>,
}

impl MapOracleImpl {
    pub fn new(
        dimensions: MapDimensions,
        tiles: HashMap<Position, StaticTile>,
        initial_entities: Vec<InitialEntitySpec>,
    ) -> Self {
        Self {
            dimensions,
            tiles,
            initial_entities,
        }
    }

    /// Creates a simple test map (all floor tiles) with sample entities
    pub fn test_map(width: u32, height: u32) -> Self {
        let dimensions = MapDimensions::new(width, height);
        let mut tiles = HashMap::new();

        // Fill with floor tiles
        for x in 0..width as i32 {
            for y in 0..height as i32 {
                tiles.insert(Position::new(x, y), StaticTile::new(TerrainKind::Floor));
            }
        }

        let initial_entities = vec![
            // Player at origin
            InitialEntitySpec {
                id: EntityId::PLAYER,
                position: Position::new(0, 0),
                kind: InitialEntityKind::Player,
            },
            // Goblin NPC at (5, 5) using template 0
            InitialEntitySpec {
                id: EntityId(1),
                position: Position::new(5, 5),
                kind: InitialEntityKind::Npc { template: 0 },
            },
        ];

        Self::new(dimensions, tiles, initial_entities)
    }
}

impl MapOracle for MapOracleImpl {
    fn dimensions(&self) -> MapDimensions {
        self.dimensions
    }

    fn tile(&self, position: Position) -> Option<StaticTile> {
        self.tiles.get(&position).copied()
    }

    fn initial_entities(&self) -> Vec<InitialEntitySpec> {
        self.initial_entities.clone()
    }
}
