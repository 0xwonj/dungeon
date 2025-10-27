//! Static dungeon layout served through [`game_core::MapOracle`].
use game_core::{MapDimensions, MapOracle, Position, StaticTile, TerrainKind};
use std::collections::HashMap;

/// MapOracle implementation with static map data (terrain only).
///
/// Holds immutable map structure that doesn't change during gameplay.
/// Entity placement is handled separately via Scenario.
pub struct MapOracleImpl {
    dimensions: MapDimensions,
    tiles: HashMap<Position, StaticTile>,
}

impl MapOracleImpl {
    pub fn new(dimensions: MapDimensions, tiles: HashMap<Position, StaticTile>) -> Self {
        Self { dimensions, tiles }
    }

    /// Creates a simple test map (all floor tiles, no entities)
    pub fn test_map(width: u32, height: u32) -> Self {
        let dimensions = MapDimensions::new(width, height);
        let mut tiles = HashMap::new();

        // Fill with floor tiles
        for x in 0..width as i32 {
            for y in 0..height as i32 {
                tiles.insert(Position::new(x, y), StaticTile::new(TerrainKind::Floor));
            }
        }

        Self::new(dimensions, tiles)
    }
}

impl MapOracle for MapOracleImpl {
    fn dimensions(&self) -> MapDimensions {
        self.dimensions
    }

    fn tile(&self, position: Position) -> Option<StaticTile> {
        self.tiles.get(&position).copied()
    }
}
