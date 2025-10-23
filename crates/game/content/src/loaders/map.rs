//! Map data loader.
//!
//! Loads pure terrain/tile data from map RON files.
//! Entity placement is handled separately via scenario files.

use std::collections::HashMap;
use std::path::Path;

use game_core::{MapDimensions, Position, StaticTile, TerrainKind};
use serde::{Deserialize, Serialize};

use crate::loaders::{LoadResult, read_file};

/// Map data structure for RON files (terrain only).
#[derive(Debug, Clone, Serialize, Deserialize)]
struct MapDataRon {
    dimensions: (u32, u32),
    tiles: Vec<(i32, i32, TerrainKind)>, // (x, y, terrain)
}

/// Loader for map data from RON files.
pub struct MapLoader;

impl MapLoader {
    /// Load map data from a RON file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the RON file containing MapData
    ///
    /// # Returns
    ///
    /// Returns dimensions and tiles HashMap (terrain only, no entities).
    pub fn load(path: &Path) -> LoadResult<(MapDimensions, HashMap<Position, StaticTile>)> {
        let content = read_file(path)?;
        let data: MapDataRon = ron::from_str(&content)
            .map_err(|e| anyhow::anyhow!("Failed to parse map RON: {}", e))?;

        let dimensions = MapDimensions::new(data.dimensions.0, data.dimensions.1);

        // Fill entire map with default Floor tiles first
        let mut tiles = HashMap::new();
        for y in 0..dimensions.height {
            for x in 0..dimensions.width {
                let pos = Position::new(x as i32, y as i32);
                tiles.insert(pos, StaticTile::new(TerrainKind::Floor));
            }
        }

        // Then override with explicitly defined tiles
        for (x, y, terrain) in data.tiles {
            let pos = Position::new(x, y);
            tiles.insert(pos, StaticTile::new(terrain));
        }

        Ok((dimensions, tiles))
    }
}
