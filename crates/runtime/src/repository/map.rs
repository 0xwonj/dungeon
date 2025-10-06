use game_core::{EntityId, MapDimensions, Position, StaticTile, TerrainKind};
use std::collections::HashMap;

use super::MapRepository;
use crate::error::Result;

/// In-memory implementation of MapRepository
pub struct InMemoryMapRepo {
    dimensions: MapDimensions,
    tiles: HashMap<Position, StaticTile>,
    entities: HashMap<EntityId, Position>,
}

impl InMemoryMapRepo {
    pub fn new(dimensions: MapDimensions) -> Self {
        Self {
            dimensions,
            tiles: HashMap::new(),
            entities: HashMap::new(),
        }
    }

    /// Add a tile to the map
    pub fn add_tile(&mut self, pos: Position, tile: StaticTile) {
        self.tiles.insert(pos, tile);
    }

    /// Register an entity at a position
    pub fn add_entity(&mut self, entity: EntityId, pos: Position) {
        self.entities.insert(entity, pos);
    }

    /// Create a simple test map (all floor tiles)
    pub fn test_map(width: u32, height: u32) -> Self {
        let mut repo = Self::new(MapDimensions::new(width, height));

        // Fill with floor tiles
        for x in 0..width as i32 {
            for y in 0..height as i32 {
                repo.add_tile(
                    Position::new(x, y),
                    StaticTile::new(TerrainKind::Floor),
                );
            }
        }

        repo
    }
}

impl MapRepository for InMemoryMapRepo {
    fn get_tile(&self, pos: Position) -> Result<Option<StaticTile>> {
        Ok(self.tiles.get(&pos).copied())
    }

    fn get_nearby_entities(
        &self,
        center: Position,
        radius: u32,
    ) -> Result<Vec<(EntityId, Position)>> {
        let mut result = Vec::new();

        for (entity, pos) in &self.entities {
            let dx = (pos.x - center.x).abs() as u32;
            let dy = (pos.y - center.y).abs() as u32;

            if dx <= radius && dy <= radius {
                result.push((*entity, *pos));
            }
        }

        Ok(result)
    }
}
