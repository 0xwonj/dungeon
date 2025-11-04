//! Available actions query system.
//!
//! This module provides functionality to query which actions an entity can
//! currently perform based on their action abilities and cooldown state.

use crate::action::ActionKind;
use crate::env::GameEnv;
use crate::state::{EntityId, GameState};

/// Get all actions currently available to an entity.
///
/// Returns all actions from the entity's ability list that are:
/// - Enabled (`enabled = true`)
/// - Not on cooldown (`cooldown_until <= current_tick`)
///
/// Returns an empty vec if the entity doesn't exist or is not an actor.
pub fn get_available_actions(
    entity: EntityId,
    state: &GameState,
    _env: &GameEnv<'_>,
) -> Vec<ActionKind> {
    let Some(actor) = state.entities.actor(entity) else {
        return Vec::new();
    };

    let current_tick = state.turn.clock;

    actor
        .actions
        .iter()
        .filter(|ability| ability.is_ready(current_tick))
        .map(|ability| ability.kind)
        .collect()
}
