pub mod common;
pub mod entities;
pub mod turn;
pub mod world;

pub use common::{EntityId, Position, ResourceMeter};
pub use entities::{
    ActorState, ActorStats, EntitiesState, InventoryState, ItemHandle, ItemState, PropKind,
    PropState,
};
pub use turn::{TurnPhase, TurnState};
pub use world::{TileMap, TileOverlay, TileState, WorldState};

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
}
