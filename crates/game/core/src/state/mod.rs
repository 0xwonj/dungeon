//! Authoritative game state representation.
//!
//! This module owns the data structures that describe entities, turn
//! bookkeeping, overlays, and initialization helpers. Runtime layers clone or
//! query this state but mutate it exclusively through the engine.
pub mod delta;
pub mod types;

use crate::env::{GameEnv, InitialEntityKind, MapOracle};
pub use bounded_vector::BoundedVec;
pub use delta::{
    ActorPatch, CollectionDelta, EntitiesDelta, ItemPatch, OccupancyPatch, OverlayPatch, PropPatch,
    StateDelta, TurnDelta, WorldDelta,
};
pub use types::{
    ActorState, ActorStats, EntitiesState, EntityId, EventId, HazardOverlay, InventoryState,
    ItemHandle, ItemState, Overlay, OverlaySet, Position, PropKind, PropState, ResourceMeter, Tick,
    TileMap, TileView, TurnState, WorldState,
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
        M: MapOracle + ?Sized,
    {
        self.world.tile_view(map, position)
    }

    /// Determines whether a tile can be entered considering terrain passability and occupancy.
    pub fn can_enter<M>(&self, map: &M, position: Position) -> bool
    where
        M: MapOracle + ?Sized,
    {
        self.tile_view(map, position)
            .map(|view| view.is_passable() && !view.is_occupied())
            .unwrap_or(false)
    }

    /// Creates a new GameState from initial entity specifications provided by the map oracle.
    ///
    /// This is the canonical way to initialize game state at the start of a session.
    /// The function:
    /// - Reads initial entity specs from the map oracle
    /// - Resolves NPC templates from the tables oracle
    /// - Creates all entities (player, NPCs, props, items)
    /// - Sets up tile occupancy
    ///
    /// Returns an error if required oracles are missing or entity limits are exceeded.
    pub fn from_initial_entities(env: &GameEnv<'_>) -> Result<Self, InitializationError> {
        let map = env.map().ok_or(InitializationError::MissingMapOracle)?;
        let npcs = env.npcs().ok_or(InitializationError::MissingNpcOracle)?;

        let mut state = GameState::default();
        let initial_entities = map.initial_entities();

        // Process each initial entity spec
        for spec in initial_entities {
            match spec.kind {
                InitialEntityKind::Player => {
                    // Player uses default stats for now
                    state.entities.player = ActorState::new(
                        spec.id,
                        spec.position,
                        ActorStats::default(),
                        InventoryState::default(),
                    );

                    // Activate player in turn system at tick 0
                    state.entities.player.ready_at = Some(Tick(0));
                    state.turn.active_actors.insert(spec.id);

                    // Add player to tile occupancy
                    state.world.tile_map.add_occupant(spec.position, spec.id);
                }

                InitialEntityKind::Npc { template } => {
                    // Resolve template to get stats and inventory
                    let npc_template = npcs
                        .template(template)
                        .ok_or(InitializationError::UnknownNpcTemplate(template))?;

                    let actor = ActorState::new(
                        spec.id,
                        spec.position,
                        npc_template.stats,
                        npc_template.inventory,
                    )
                    .with_template_id(template)
                    .with_ready_at(Tick(0));

                    // Activate NPC in turn system at tick 0
                    state.turn.active_actors.insert(spec.id);

                    state
                        .entities
                        .npcs
                        .push(actor)
                        .map_err(|_| InitializationError::TooManyNpcs)?;

                    // Add NPC to tile occupancy
                    state.world.tile_map.add_occupant(spec.position, spec.id);
                }

                InitialEntityKind::Prop { kind, is_active } => {
                    let prop = PropState::new(spec.id, spec.position, kind, is_active);

                    state
                        .entities
                        .props
                        .push(prop)
                        .map_err(|_| InitializationError::TooManyProps)?;

                    // Props also occupy tiles
                    state.world.tile_map.add_occupant(spec.position, spec.id);
                }

                InitialEntityKind::Item { handle } => {
                    let item = ItemState::new(spec.id, spec.position, handle);

                    state
                        .entities
                        .items
                        .push(item)
                        .map_err(|_| InitializationError::TooManyItems)?;

                    // Items don't block movement, so we don't add to occupancy
                }
            }
        }

        Ok(state)
    }
}

/// Errors that can occur during initial state creation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum InitializationError {
    #[error("map oracle not available")]
    MissingMapOracle,

    #[error("npc oracle not available")]
    MissingNpcOracle,

    #[error("unknown npc template id: {0}")]
    UnknownNpcTemplate(u16),

    #[error("too many npcs")]
    TooManyNpcs,

    #[error("too many props")]
    TooManyProps,

    #[error("too many items")]
    TooManyItems,
}
