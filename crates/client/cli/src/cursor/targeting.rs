//! Targeting filters for cursor snap-to behavior.
//!
//! Future: This module will be used when implementing Targeting mode for combat actions.

#![allow(dead_code)]

use game_core::{EntityId, GameState, Position};

/// Filter function for determining valid targets.
pub trait TargetFilter {
    fn is_valid(&self, state: &GameState, entity: EntityId) -> bool;
}

/// Filters for hostile NPCs only.
pub struct HostileFilter;

impl TargetFilter for HostileFilter {
    fn is_valid(&self, state: &GameState, entity: EntityId) -> bool {
        // For now, all non-player actors are considered hostile
        entity != EntityId::PLAYER && state.entities.actor(entity).is_some()
    }
}

/// Filters for all actors (player + NPCs).
pub struct ActorFilter;

impl TargetFilter for ActorFilter {
    fn is_valid(&self, state: &GameState, entity: EntityId) -> bool {
        state.entities.actor(entity).is_some()
    }
}

/// Collects entity positions matching a filter.
pub fn collect_targets<F: TargetFilter>(
    state: &GameState,
    filter: &F,
) -> Vec<(EntityId, Position)> {
    let mut targets = Vec::new();

    // Check all actors
    for actor in state.entities.all_actors() {
        if filter.is_valid(state, actor.id) {
            targets.push((actor.id, actor.position));
        }
    }

    // Could add props/items here if needed

    targets
}
