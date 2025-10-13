//! View-model snapshots derived from [`game_core::GameState`].
use game_core::{
    EntityId, GameState, ItemHandle, Position, PropKind,
    env::{MapOracle, TerrainKind},
};

use crate::message::{MessageEntry, MessageLog};

/// High-level snapshot of the game used by presentation layers.
#[derive(Clone, Debug)]
pub struct UiFrame {
    pub turn: TurnSummary,
    pub map: MapSnapshot,
    pub player: PlayerSnapshot,
    pub world: WorldSnapshot,
    pub messages: Vec<MessageEntry>,
}

impl UiFrame {
    pub fn from_state<M: MapOracle + ?Sized>(
        map_oracle: &M,
        state: &GameState,
        messages: &MessageLog,
        message_limit: usize,
    ) -> Self {
        Self {
            turn: TurnSummary::from_state(state),
            map: MapSnapshot::from_state(map_oracle, state),
            player: PlayerSnapshot::from_state(state),
            world: WorldSnapshot::from_state(state),
            messages: collect_messages(messages, message_limit),
        }
    }
}

#[derive(Clone, Debug)]
pub struct TurnSummary {
    pub clock: u64,
    pub current_actor: EntityId,
    pub active_actors: Vec<EntityId>,
}

impl TurnSummary {
    fn from_state(state: &GameState) -> Self {
        let mut active: Vec<_> = state.turn.active_actors.iter().copied().collect();
        active.sort();
        Self {
            clock: state.turn.clock.0,
            current_actor: state.turn.current_actor,
            active_actors: active,
        }
    }
}

#[derive(Clone, Debug)]
pub struct MapSnapshot {
    pub width: u32,
    pub height: u32,
    pub tiles: Vec<Vec<MapTile>>,
}

impl MapSnapshot {
    fn from_state<M: MapOracle + ?Sized>(map_oracle: &M, state: &GameState) -> Self {
        let dimensions = map_oracle.dimensions();
        let mut tiles = Vec::with_capacity(dimensions.height as usize);

        for y in (0..dimensions.height as i32).rev() {
            let mut row = Vec::with_capacity(dimensions.width as usize);
            for x in 0..dimensions.width as i32 {
                let position = Position::new(x, y);
                row.push(MapTile::from_state(map_oracle, state, position));
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

#[derive(Clone, Debug)]
pub struct MapTile {
    pub position: Position,
    pub terrain: TerrainKind,
    pub occupants: Vec<OccupantView>,
    pub overlays: usize,
    pub loose_items: usize,
}

/// Detailed tile information for Examine mode.
#[derive(Clone, Debug)]
pub struct TileInfoSnapshot {
    pub position: Position,
    pub terrain: TerrainKind,
    pub is_passable: bool,
    pub is_occupied: bool,
    pub entities: Vec<EntityDetailView>,
    pub overlay_count: usize,
}

/// Detailed entity view for inspection.
#[derive(Clone, Debug)]
pub enum EntityDetailView {
    Player {
        id: EntityId,
        stats: ActorStatsSnapshot,
    },
    Npc {
        id: EntityId,
        template_id: u16,
        stats: ActorStatsSnapshot,
    },
    Prop {
        id: EntityId,
        kind: PropKind,
        is_active: bool,
    },
    Item {
        id: EntityId,
        handle: ItemHandle,
        // name and category will be resolved by UI layer using ItemOracle
    },
}

impl MapTile {
    fn from_state<M: MapOracle + ?Sized>(
        map_oracle: &M,
        state: &GameState,
        position: Position,
    ) -> Self {
        let terrain = map_oracle
            .tile(position)
            .map(|tile| tile.terrain())
            .unwrap_or(TerrainKind::Void);

        let occupants = build_occupants(state, position);
        let overlays = state
            .world
            .tile_map
            .overlay(&position)
            .map_or(0, |overlay| overlay.iter().count());

        let loose_items = state
            .entities
            .items
            .iter()
            .filter(|item| item.position == position)
            .count();

        Self {
            position,
            terrain,
            occupants,
            overlays,
            loose_items,
        }
    }
}

#[derive(Clone, Debug)]
pub enum OccupantView {
    Player {
        id: EntityId,
        is_current: bool,
        stats: ActorStatsSnapshot,
    },
    Npc {
        id: EntityId,
        is_current: bool,
        template_id: u16,
        stats: ActorStatsSnapshot,
    },
    Prop {
        id: EntityId,
        kind: PropKind,
        is_active: bool,
    },
}

/// Stats snapshot for display purposes.
#[derive(Clone, Debug)]
pub struct ActorStatsSnapshot {
    pub health: ResourceSnapshot,
    pub energy: ResourceSnapshot,
    pub speed: u16,
    pub ready_at: Option<u64>,
}

#[derive(Clone, Debug)]
pub struct PlayerSnapshot {
    pub id: EntityId,
    pub position: Position,
    pub stats: PlayerStats,
    pub inventory_items: usize,
}

impl PlayerSnapshot {
    fn from_state(state: &GameState) -> Self {
        let player = &state.entities.player;
        Self {
            id: player.id,
            position: player.position,
            stats: PlayerStats {
                health: ResourceSnapshot::new(
                    player.stats.health.current,
                    player.stats.health.maximum,
                ),
                energy: ResourceSnapshot::new(
                    player.stats.energy.current,
                    player.stats.energy.maximum,
                ),
                speed: player.stats.speed,
                ready_at: player.ready_at.map(|tick| tick.0),
            },
            inventory_items: player.inventory.items.len(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct PlayerStats {
    pub health: ResourceSnapshot,
    pub energy: ResourceSnapshot,
    pub speed: u16,
    pub ready_at: Option<u64>,
}

#[derive(Clone, Debug)]
pub struct ResourceSnapshot {
    pub current: u32,
    pub maximum: u32,
}

impl ResourceSnapshot {
    fn new(current: u32, maximum: u32) -> Self {
        Self { current, maximum }
    }
}

#[derive(Clone, Debug)]
pub struct WorldSnapshot {
    pub npc_count: usize,
    pub prop_count: usize,
    pub loose_item_count: usize,
}

impl WorldSnapshot {
    fn from_state(state: &GameState) -> Self {
        Self {
            npc_count: state.entities.npcs.len(),
            prop_count: state.entities.props.len(),
            loose_item_count: state.entities.items.len(),
        }
    }
}

fn collect_messages(messages: &MessageLog, limit: usize) -> Vec<MessageEntry> {
    messages.recent(limit).cloned().collect()
}

/// Helper to create ActorStatsSnapshot from ActorState.
fn actor_stats_snapshot(actor: &game_core::ActorState) -> ActorStatsSnapshot {
    ActorStatsSnapshot {
        health: ResourceSnapshot::new(actor.stats.health.current, actor.stats.health.maximum),
        energy: ResourceSnapshot::new(actor.stats.energy.current, actor.stats.energy.maximum),
        speed: actor.stats.speed,
        ready_at: actor.ready_at.map(|tick| tick.0),
    }
}

impl TileInfoSnapshot {
    /// Creates a detailed tile snapshot for Examine mode.
    pub fn from_state<M: MapOracle + ?Sized>(
        map_oracle: &M,
        state: &GameState,
        position: Position,
    ) -> Self {
        let terrain = map_oracle
            .tile(position)
            .map(|tile| tile.terrain())
            .unwrap_or(TerrainKind::Void);

        let tile_view = state.tile_view(map_oracle, position);
        let is_passable = tile_view.as_ref().is_some_and(|v| v.is_passable());
        let is_occupied = tile_view.as_ref().is_some_and(|v| v.is_occupied());

        let mut entities = Vec::new();

        // Add occupants (actors and props)
        if let Some(ids) = state.world.tile_map.occupants(&position) {
            for id in ids {
                if *id == state.entities.player.id {
                    entities.push(EntityDetailView::Player {
                        id: *id,
                        stats: actor_stats_snapshot(&state.entities.player),
                    });
                    continue;
                }

                if let Some(npc) = state.entities.npcs.iter().find(|npc| npc.id == *id) {
                    entities.push(EntityDetailView::Npc {
                        id: npc.id,
                        template_id: npc.template_id,
                        stats: actor_stats_snapshot(npc),
                    });
                    continue;
                }

                if let Some(prop) = state.entities.props.iter().find(|prop| prop.id == *id) {
                    entities.push(EntityDetailView::Prop {
                        id: prop.id,
                        kind: prop.kind.clone(),
                        is_active: prop.is_active,
                    });
                }
            }
        }

        // Add loose items on this tile
        for item in state.entities.items.iter() {
            if item.position == position {
                entities.push(EntityDetailView::Item {
                    id: item.id,
                    handle: item.handle,
                });
            }
        }

        let overlay_count = state
            .world
            .tile_map
            .overlay(&position)
            .map_or(0, |overlay| overlay.iter().count());

        Self {
            position,
            terrain,
            is_passable,
            is_occupied,
            entities,
            overlay_count,
        }
    }
}

fn build_occupants(state: &GameState, position: Position) -> Vec<OccupantView> {
    let mut occupants = Vec::new();

    if let Some(ids) = state.world.tile_map.occupants(&position) {
        for id in ids {
            if *id == state.entities.player.id {
                occupants.push(OccupantView::Player {
                    id: *id,
                    is_current: state.turn.current_actor == *id,
                    stats: actor_stats_snapshot(&state.entities.player),
                });
                continue;
            }

            if let Some(npc) = state.entities.npcs.iter().find(|npc| npc.id == *id) {
                occupants.push(OccupantView::Npc {
                    id: npc.id,
                    is_current: state.turn.current_actor == npc.id,
                    template_id: npc.template_id,
                    stats: actor_stats_snapshot(npc),
                });
                continue;
            }

            if let Some(prop) = state.entities.props.iter().find(|prop| prop.id == *id) {
                occupants.push(OccupantView::Prop {
                    id: prop.id,
                    kind: prop.kind.clone(),
                    is_active: prop.is_active,
                });
            }
        }
    }

    occupants
}
