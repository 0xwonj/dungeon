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
        // For now, all NPCs are considered hostile
        state.entities.npcs.iter().any(|npc| npc.id == entity)
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

    // Check player
    if filter.is_valid(state, state.entities.player.id) {
        targets.push((state.entities.player.id, state.entities.player.position));
    }

    // Check NPCs
    for npc in state.entities.npcs.iter() {
        if filter.is_valid(state, npc.id) {
            targets.push((npc.id, npc.position));
        }
    }

    // Could add props/items here if needed

    targets
}
