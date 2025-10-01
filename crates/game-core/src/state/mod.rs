pub mod common;
pub mod entities;
pub mod turn;
pub mod world;

use crate::env::MapOracle;
pub use common::{EntityId, Position, ResourceMeter};
pub use entities::{
    ActorState, ActorStats, EntitiesState, InventoryState, ItemHandle, ItemState, PropKind,
    PropState,
};
pub use turn::{TurnPhase, TurnState};
pub use world::{
    EventId, HazardOverlay, OccupancyIndex, Overlay, OverlaySet, TileMap, TileView, WorldState,
};

/// Canonical snapshot of the deterministic game state.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct GameState {
    /// Turn bookkeeping including current phase within the turn.
    pub turn: TurnState,
    /// All entities tracked in the room: actors, props, items.
    pub entities: EntitiesState,
    /// Runtime world data layered on top of the static map commitment.
    pub world: WorldState,
}

impl GameState {
    /// Creates a fresh state from the provided sub-components.
    pub fn new(turn: TurnState, entities: EntitiesState, world: WorldState) -> Self {
        Self {
            turn,
            entities,
            world,
        }
    }

    /// Returns a merged tile view that combines static map data with runtime overlays.
    pub fn tile_view<'a, M>(&'a self, map: &M, position: Position) -> Option<TileView<'a>>
    where
        M: MapOracle,
    {
        self.world.tile_view(map, position)
    }

    /// Determines whether a tile can be entered considering terrain passability and occupancy.
    pub fn can_enter<M>(&self, map: &M, position: Position) -> bool
    where
        M: MapOracle,
    {
        self.tile_view(map, position)
            .map(|view| view.is_passable() && !view.is_occupied())
            .unwrap_or(false)
    }
}
