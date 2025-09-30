use std::collections::BTreeMap;

use super::{EntityId, Position};

/// Aggregated world-level state layered on top of the static tile map commitment.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct WorldState {
    pub tile_map: TileMap,
}

impl WorldState {
    pub fn new(tile_map: TileMap) -> Self {
        Self { tile_map }
    }
}

/// Dynamic tile snapshot storing runtime deltas and occupants per coordinate.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct TileMap {
    /// Sparse set of tiles whose runtime state differs from the static map.
    pub tiles: BTreeMap<Position, TileState>,
}

impl TileMap {
    pub fn new(tiles: BTreeMap<Position, TileState>) -> Self {
        Self { tiles }
    }
}

/// Tile-level state capturing both overlay effects and current occupants.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct TileState {
    pub overlay: Option<TileOverlay>,
    pub occupants: Vec<EntityId>,
}

impl TileState {
    pub fn new(overlay: Option<TileOverlay>, occupants: Vec<EntityId>) -> Self {
        Self { overlay, occupants }
    }
}

/// Representation of how a tile differs from the immutable base map.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TileOverlay {
    Door { open: bool },
    Hazard { remaining_turns: u32 },
}
