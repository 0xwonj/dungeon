//! Entity view types for presentation.
//!
//! These types provide presentation-optimized views of game entities,
//! directly reusing game-core types to avoid duplication between ZK and UI.

use game_core::{EntityId, ItemHandle, Position, PropKind, Tick, stats::StatsSnapshot};

/// Actor view (Player + NPCs) for rendering and targeting.
///
/// Uses `game_core::StatsSnapshot` directly - the same type used in ZK proofs.
#[derive(Clone, Debug)]
pub struct ActorView {
    pub id: EntityId,
    pub position: Position,
    pub is_player: bool,
    /// Complete stats snapshot from game-core.
    /// Use `.hp()`, `.mp()` methods to get current/max values.
    pub stats: StatsSnapshot,
    /// When this actor is scheduled to act next.
    /// - `Some(tick)`: Actor will act at this tick
    /// - `None`: Actor is not currently scheduled (outside activation radius)
    pub ready_at: Option<Tick>,
}

impl ActorView {
    pub fn from_actor(actor: &game_core::ActorState) -> Self {
        Self {
            id: actor.id,
            position: actor.position,
            is_player: actor.id == EntityId::PLAYER,
            stats: actor.snapshot(),
            ready_at: actor.ready_at,
        }
    }
}

/// Prop view for examination and rendering.
#[derive(Clone, Debug)]
pub struct PropView {
    pub id: EntityId,
    pub position: Position,
    pub kind: PropKind,
    pub is_active: bool,
}

impl PropView {
    pub fn from_prop(prop: &game_core::PropState) -> Self {
        Self {
            id: prop.id,
            position: prop.position,
            kind: prop.kind.clone(),
            is_active: prop.is_active,
        }
    }
}

/// Item view for examination and rendering.
#[derive(Clone, Debug)]
pub struct ItemView {
    pub id: EntityId,
    pub position: Position,
    pub handle: ItemHandle,
}

impl ItemView {
    pub fn from_item(item: &game_core::ItemState) -> Self {
        Self {
            id: item.id,
            position: item.position,
            handle: item.handle,
        }
    }
}

/// Get the player actor from game state.
///
/// # Panics
/// Panics if player entity does not exist (should never happen in valid game state).
pub fn get_player(state: &game_core::GameState) -> ActorView {
    state
        .entities
        .all_actors()
        .find(|actor| actor.id == game_core::EntityId::PLAYER)
        .map(ActorView::from_actor)
        .expect("Player entity must exist in game state")
}

/// Collect all NPC actors from game state (excludes player).
pub fn collect_npcs(state: &game_core::GameState) -> Vec<ActorView> {
    state
        .entities
        .all_actors()
        .filter(|actor| actor.id != game_core::EntityId::PLAYER)
        .map(ActorView::from_actor)
        .collect()
}

/// Collect all actors from game state (player + NPCs).
///
/// # Invariant
///
/// Returns a Vec where the first element is always the player (EntityId::PLAYER).
/// This ensures `actors[0]` can be used as a cached player reference.
pub fn collect_actors(state: &game_core::GameState) -> Vec<ActorView> {
    let mut actors: Vec<ActorView> = state
        .entities
        .all_actors()
        .map(ActorView::from_actor)
        .collect();

    // Ensure player is always first
    actors.sort_by_key(|a| {
        if a.id == game_core::EntityId::PLAYER {
            0
        } else {
            1
        }
    });

    actors
}

/// Collect all props from game state.
pub fn collect_props(state: &game_core::GameState) -> Vec<PropView> {
    state
        .entities
        .props
        .iter()
        .map(PropView::from_prop)
        .collect()
}

/// Collect all items from game state.
pub fn collect_items(state: &game_core::GameState) -> Vec<ItemView> {
    state
        .entities
        .items
        .iter()
        .map(ItemView::from_item)
        .collect()
}
