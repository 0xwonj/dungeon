//! Post-execution hooks that apply additional state changes after action execution.
//!
//! Hooks are triggered based on the state delta produced by an action, allowing for
//! automatic side-effects like entity activation/deactivation, regeneration, environmental
//! effects, etc.

use std::sync::Arc;

use crate::engine::StateReducer;
use crate::env::GameEnv;
use crate::state::StateDelta;

/// A hook that is applied after an action has been executed.
///
/// Hooks can inspect the state delta and conditionally apply additional
/// state changes through a [`StateReducer`].
///
/// Hooks are executed in priority order (lower priority values execute first).
pub trait PostExecutionHook: Send + Sync {
    /// Returns the priority of this hook. Lower values execute first.
    /// Default priority is 0.
    fn priority(&self) -> i32 {
        0
    }

    /// Determines whether this hook should be triggered based on the state delta.
    fn should_trigger(&self, delta: &StateDelta) -> bool;

    /// Applies the hook's effects to the game state via a reducer.
    fn apply(&self, reducer: &mut StateReducer<'_>, delta: &StateDelta, env: &GameEnv<'_>);
}

/// Hook that maintains the active entity set based on player proximity.
///
/// When the player moves, this hook activates entities within the activation radius
/// and deactivates entities outside of it.
#[derive(Debug)]
pub struct ActivationHook;

impl PostExecutionHook for ActivationHook {
    fn priority(&self) -> i32 {
        // Run early so entities are activated before other hooks might need them
        -10
    }

    fn should_trigger(&self, delta: &StateDelta) -> bool {
        // Trigger if player position changed
        delta
            .entities
            .player
            .as_ref()
            .and_then(|patch| patch.position)
            .is_some()
    }

    fn apply(&self, reducer: &mut StateReducer<'_>, _delta: &StateDelta, env: &GameEnv<'_>) {
        let activation_radius = env.activation_radius();
        let state = reducer.state();
        let player_position = state.entities.player.position;
        let clock = state.turn.clock;

        // Collect NPC positions and current activation status
        let npc_data: Vec<_> = state
            .entities
            .npcs
            .iter()
            .map(|npc| {
                let is_active = state.turn.active_actors.contains(&npc.id);
                (npc.id, npc.position, is_active, npc.stats.clone())
            })
            .collect();

        // Calculate which entities should be activated/deactivated
        let mut to_activate = Vec::new();
        let mut to_deactivate = Vec::new();

        for (entity_id, npc_position, is_active, stats) in npc_data {
            let dx = (npc_position.x - player_position.x).unsigned_abs();
            let dy = (npc_position.y - player_position.y).unsigned_abs();

            if dx <= activation_radius && dy <= activation_radius {
                if !is_active {
                    let delay = crate::action::Action::calculate_delay(
                        &crate::action::ActionKind::Wait,
                        &stats,
                    );
                    to_activate.push((entity_id, delay));
                }
            } else if is_active {
                to_deactivate.push(entity_id);
            }
        }

        // Apply changes via reducer
        {
            let mut turn = reducer.turn();
            for (entity_id, _) in &to_activate {
                turn.activate(*entity_id);
            }
            for entity_id in &to_deactivate {
                turn.deactivate(*entity_id);
            }
        }

        {
            let mut entities = reducer.entities();
            for (entity_id, delay) in to_activate {
                entities.set_actor_ready_at(entity_id, Some(crate::state::Tick(clock.0 + delay.0)));
            }
            for entity_id in to_deactivate {
                entities.set_actor_ready_at(entity_id, None);
            }
        }
    }
}

/// Returns the default set of hooks that should be applied after every action execution.
/// Hooks are returned in an Arc for efficient sharing without cloning.
pub fn default_hooks() -> Arc<[Arc<dyn PostExecutionHook>]> {
    let mut hooks: Vec<Arc<dyn PostExecutionHook>> = vec![Arc::new(ActivationHook)];

    // Sort by priority (lower values first)
    hooks.sort_by_key(|h| h.priority());

    hooks.into()
}
