//! Map view types for 2D grid rendering.

use game_core::{
    GameState, Position,
    env::{MapOracle, TerrainKind},
};

/// 2D map view optimized for grid rendering.
#[derive(Clone, Debug)]
pub struct MapView {
    pub width: u32,
    pub height: u32,
    /// Tiles in row-major order, Y-reversed (top row first for rendering).
    pub tiles: Vec<Vec<TileView>>,
}

impl MapView {
    pub fn from_state<M: MapOracle + ?Sized>(map_oracle: &M, state: &GameState) -> Self {
        let dimensions = map_oracle.dimensions();
        let mut tiles = Vec::with_capacity(dimensions.height as usize);

        // Iterate Y in reverse for top-to-bottom rendering
        for y in (0..dimensions.height as i32).rev() {
            let mut row = Vec::with_capacity(dimensions.width as usize);
            for x in 0..dimensions.width as i32 {
                let position = Position::new(x, y);
                row.push(TileView::from_state(map_oracle, state, position));
            }
            tiles.push(row);
        }

        Self {
            width: dimensions.width,
            height: dimensions.height,
            tiles,
        }
    }
}

/// Single tile in the map view.
#[derive(Clone, Debug)]
pub struct TileView {
    pub position: Position,
    pub terrain: TerrainKind,
}

impl TileView {
    fn from_state<M: MapOracle + ?Sized>(
        map_oracle: &M,
        _state: &GameState,
        position: Position,
    ) -> Self {
        let terrain = map_oracle
            .tile(position)
            .map(|tile| tile.terrain())
            .unwrap_or(TerrainKind::Void);

        Self { position, terrain }
    }
}
