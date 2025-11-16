//! Action validation logic.
//!
//! Pre and post validation for action execution.

use crate::action::TargetingMode;
use crate::action::error::ActionError;
use crate::action::profile::ActionProfile;
use crate::action::types::{ActionInput, CharacterAction};
use crate::env::GameEnv;
use crate::state::{ActorState, GameState, Position};
use crate::stats::ResourceKind;

/// Pre-validation: Check requirements before executing.
pub(super) fn pre_validate(
    action: &CharacterAction,
    state: &GameState,
    env: &GameEnv<'_>,
) -> Result<(), ActionError> {
    // 1. Check actor exists
    let actor = state
        .entities
        .actor(action.actor)
        .ok_or(ActionError::ActorNotFound)?;

    // 2. Check if it's this actor's turn
    if state.turn.current_actor != action.actor {
        return Err(ActionError::NotActorsTurn);
    }

    // 3. Check if actor is ready to act (scheduled and time has come)
    let current_tick = state.turn.clock;
    if let Some(ready_at) = actor.ready_at {
        if ready_at > current_tick {
            return Err(ActionError::ActorNotReady);
        }
    } else {
        // Actor has no ready_at time set - not scheduled
        return Err(ActionError::ActorNotReady);
    }

    // 4. Check resources (HP, MP, Lucidity)
    validate_resources(actor)?;

    // 5. Check action ability availability
    // Verify that the actor has this action and it's usable (enabled + not on cooldown)
    if !actor.can_use_action(action.kind, current_tick) {
        return Err(ActionError::ActionNotAvailable);
    }

    // 6. Load action profile
    let profile = env
        .actions()
        .map_err(|_| ActionError::ProfileNotFound)?
        .action_profile(action.kind);

    // 7. Check resource costs
    validate_resource_costs(actor, &profile)?;

    // 8. Validate target based on targeting mode
    validate_targeting(action, state, env, &profile.targeting)?;

    Ok(())
}

/// Validate that actor has sufficient resources to survive.
///
/// Checks:
/// - HP > 0 (actor is alive)
/// - MP >= 0 (valid state)
/// - Lucidity >= 0 (valid state)
fn validate_resources(actor: &ActorState) -> Result<(), ActionError> {
    // Check HP (must be alive)
    if actor.resources.hp == 0 {
        return Err(ActionError::ActorDead);
    }

    Ok(())
}

/// Validate that actor can afford the resource costs of this action.
///
/// Checks each resource cost in the action profile against the actor's
/// current resource values.
fn validate_resource_costs(actor: &ActorState, profile: &ActionProfile) -> Result<(), ActionError> {
    for cost in &profile.resource_costs {
        let current = match cost.resource {
            ResourceKind::Hp => actor.resources.hp,
            ResourceKind::Mp => actor.resources.mp,
            ResourceKind::Lucidity => actor.resources.lucidity,
        };

        if current < cost.amount {
            return Err(ActionError::InsufficientResources);
        }
    }

    Ok(())
}

/// Validate targeting based on action's targeting mode.
fn validate_targeting(
    action: &CharacterAction,
    state: &GameState,
    _env: &GameEnv<'_>,
    targeting: &TargetingMode,
) -> Result<(), ActionError> {
    match targeting {
        TargetingMode::None | TargetingMode::SelfOnly => {
            // No target validation needed
            Ok(())
        }

        TargetingMode::SingleTarget {
            range,
            requires_los: _,
        } => {
            // Must have a single entity input
            let target_id = match action.input {
                ActionInput::Entity(id) => id,
                _ => return Err(ActionError::InvalidTarget),
            };

            // Check range (Chebyshev distance)
            let actor_pos = state
                .actor_position(action.actor)
                .ok_or(ActionError::ActorNotFound)?;
            let target_pos = state
                .actor_position(target_id)
                .ok_or(ActionError::TargetNotFound)?;
            let distance = calculate_distance(actor_pos, target_pos);
            if distance > *range {
                return Err(ActionError::OutOfRange);
            }

            Ok(())
        }

        TargetingMode::Directional { range: _, width: _ } => {
            // Must have a direction input
            match action.input {
                ActionInput::Direction(_) => Ok(()),
                _ => Err(ActionError::InvalidTarget),
            }
        }
    }
}

/// Calculate Chebyshev distance (chessboard distance) between two positions.
///
/// This is `max(|dx|, |dy|)`, which treats diagonal movement as having the same
/// cost as orthogonal movement (like a chess king).
fn calculate_distance(from: Position, to: Position) -> u32 {
    let dx = (from.x - to.x).abs();
    let dy = (from.y - to.y).abs();
    dx.max(dy) as u32
}

/// Post-validation: Check invariants after execution.
///
/// ## Future Validations
/// - HP/MP/Lucidity don't exceed maximums
/// - No entities with invalid states
/// - Death state propagation
/// - Status effect consistency
///
/// Currently unimplemented (always returns Ok).
pub(super) fn post_validate(
    _action: &CharacterAction,
    _state: &GameState,
    _env: &GameEnv<'_>,
) -> Result<(), ActionError> {
    // TODO: Implement post-validation
    Ok(())
}
