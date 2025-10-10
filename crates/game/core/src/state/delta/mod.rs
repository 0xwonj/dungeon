mod collection;
mod patch;

use std::collections::BTreeSet;

use crate::action::Action;
use crate::state::types::Overlay;
use crate::state::types::{ActorState, ItemState, PropState};
use crate::state::{EntitiesState, EntityId, GameState, Position, Tick, TurnState, WorldState};

pub use collection::CollectionDelta;
pub use patch::{ActorPatch, ItemPatch, PropPatch};

use collection::diff_collection;

/// Minimal description of an executed action's impact on the deterministic state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StateDelta {
    pub action: Action,
    pub turn: TurnDelta,
    pub entities: EntitiesDelta,
    pub world: WorldDelta,
}

impl StateDelta {
    pub fn from_states(action: Action, before: &GameState, after: &GameState) -> Self {
        Self {
            action,
            turn: TurnDelta::from_states(&before.turn, &after.turn),
            entities: EntitiesDelta::from_states(&before.entities, &after.entities),
            world: WorldDelta::from_states(&before.world, &after.world),
        }
    }
}

/// Delta for [`TurnState`].
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct TurnDelta {
    pub clock: Option<Tick>,
    pub current_actor: Option<EntityId>,
    pub activated: Vec<EntityId>,
    pub deactivated: Vec<EntityId>,
}

impl TurnDelta {
    fn from_states(before: &TurnState, after: &TurnState) -> Self {
        let clock = if before.clock != after.clock {
            Some(after.clock)
        } else {
            None
        };

        let current_actor = if before.current_actor != after.current_actor {
            Some(after.current_actor)
        } else {
            None
        };

        let activated = after
            .active_actors
            .difference(&before.active_actors)
            .copied()
            .collect();
        let deactivated = before
            .active_actors
            .difference(&after.active_actors)
            .copied()
            .collect();

        Self {
            clock,
            current_actor,
            activated,
            deactivated,
        }
    }
}

/// Delta for [`EntitiesState`].
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct EntitiesDelta {
    pub player: Option<ActorPatch>,
    pub npcs: CollectionDelta<EntityId, ActorState, ActorPatch>,
    pub props: CollectionDelta<EntityId, PropState, PropPatch>,
    pub items: CollectionDelta<EntityId, ItemState, ItemPatch>,
}

impl EntitiesDelta {
    fn from_states(before: &EntitiesState, after: &EntitiesState) -> Self {
        let player = ActorPatch::from_states(&before.player, &after.player);

        let npcs = diff_collection(
            &before.npcs,
            &after.npcs,
            |actor| actor.id,
            ActorPatch::from_states,
        );
        let props = diff_collection(
            &before.props,
            &after.props,
            |prop| prop.id,
            PropPatch::from_states,
        );
        let items = diff_collection(
            &before.items,
            &after.items,
            |item| item.id,
            ItemPatch::from_states,
        );

        Self {
            player,
            npcs,
            props,
            items,
        }
    }
}

/// Delta for [`WorldState`].
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct WorldDelta {
    pub occupancy: Vec<OccupancyPatch>,
    pub overlays: Vec<OverlayPatch>,
}

impl WorldDelta {
    fn from_states(before: &WorldState, after: &WorldState) -> Self {
        let occupancy = diff_occupancy(before, after);
        let overlays = diff_overlays(before, after);

        Self {
            occupancy,
            overlays,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OccupancyPatch {
    pub position: Position,
    pub occupants: Vec<EntityId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OverlayPatch {
    pub position: Position,
    pub overlays: Vec<Overlay>,
}

fn diff_occupancy(before: &WorldState, after: &WorldState) -> Vec<OccupancyPatch> {
    let mut positions = BTreeSet::new();
    positions.extend(before.tile_map.occupancy().keys().copied());
    positions.extend(after.tile_map.occupancy().keys().copied());

    positions
        .into_iter()
        .filter_map(|position| {
            let before_vec = before
                .tile_map
                .occupants(&position)
                .map(|slot| slot.iter().copied().collect::<Vec<_>>())
                .unwrap_or_default();
            let after_vec = after
                .tile_map
                .occupants(&position)
                .map(|slot| slot.iter().copied().collect::<Vec<_>>())
                .unwrap_or_default();

            if before_vec == after_vec {
                None
            } else {
                Some(OccupancyPatch {
                    position,
                    occupants: after_vec,
                })
            }
        })
        .collect()
}

fn diff_overlays(before: &WorldState, after: &WorldState) -> Vec<OverlayPatch> {
    let mut positions = BTreeSet::new();
    positions.extend(before.tile_map.overlays().keys().copied());
    positions.extend(after.tile_map.overlays().keys().copied());

    positions
        .into_iter()
        .filter_map(|position| {
            let before_vec = overlay_vec(before.tile_map.overlay(&position));
            let after_vec = overlay_vec(after.tile_map.overlay(&position));

            if before_vec == after_vec {
                None
            } else {
                Some(OverlayPatch {
                    position,
                    overlays: after_vec,
                })
            }
        })
        .collect()
}

fn overlay_vec(set: Option<&crate::state::types::OverlaySet>) -> Vec<Overlay> {
    match set {
        Some(overlay_set) => overlay_set.iter().cloned().collect(),
        None => Vec::new(),
    }
}
