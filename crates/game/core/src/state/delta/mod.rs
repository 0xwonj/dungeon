mod collection;
mod patch;

use std::collections::BTreeSet;

use crate::action::Action;
use crate::state::types::{ActorState, ItemState, PropState};
use crate::state::{EntitiesState, EntityId, GameState, Tick, TurnState, WorldState};

pub use collection::CollectionDelta;
pub use patch::{ActorPatch, FieldDelta, ItemPatch, OccupancyPatch, PropPatch};

use collection::diff_collection;

/// Minimal description of an executed action's impact on the deterministic state.
///
/// The delta system computes granular patches to track exactly which fields changed.
/// This design supports:
/// - **ZK proof generation**: Efficiently encode only state changes in the proof circuit
/// - **Bandwidth optimization**: Transmit minimal diffs over network
/// - **Audit trails**: Capture precise state transitions for replay and debugging
///
/// The patches use [`FieldDelta`] to clearly distinguish between unchanged fields
/// and fields that changed to a new value (including `None` for optional fields).
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
///
/// Tracks changes to all game entities (player, NPCs, props, items) using
/// granular patches that only include modified fields.
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
}

impl WorldDelta {
    fn from_states(before: &WorldState, after: &WorldState) -> Self {
        let occupancy = diff_occupancy(before, after);

        Self { occupancy }
    }
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
