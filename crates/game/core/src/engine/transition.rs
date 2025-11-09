//! Action transition dispatch and execution logic.

use crate::action::{Action, ActionResult, ActionTransition, SystemActionKind, execute};
use crate::env::GameEnv;
use crate::state::GameState;

use super::errors::{ExecuteError, TransitionPhase, TransitionPhaseError};

/// Executes a transition through the three-phase pipeline and returns the result.
///
/// Phases:
/// 1. `pre_validate` - Check preconditions before mutation
/// 2. `apply` - Mutate the game state and return result
/// 3. `post_validate` - Verify postconditions after mutation
#[inline]
fn drive_transition<T>(
    transition: &T,
    state: &mut GameState,
    env: &GameEnv<'_>,
) -> Result<T::Result, TransitionPhaseError<T::Error>>
where
    T: ActionTransition,
{
    transition
        .pre_validate(state, env)
        .map_err(|error| TransitionPhaseError::new(TransitionPhase::PreValidate, error))?;

    let result = transition
        .apply(state, env)
        .map_err(|error| TransitionPhaseError::new(TransitionPhase::Apply, error))?;

    transition
        .post_validate(state, env)
        .map_err(|error| TransitionPhaseError::new(TransitionPhase::PostValidate, error))?;

    Ok(result)
}

/// Executes an action through the transition pipeline and returns Option<ActionResult>.
///
/// This is the internal implementation used by GameEngine::execute().
/// Routes each action type to its transition.
/// Returns Some(ActionResult) for character actions, None for system actions.
pub(super) fn execute_transition(
    action: &Action,
    state: &mut GameState,
    env: &GameEnv<'_>,
) -> Result<Option<ActionResult>, ExecuteError> {
    match action {
        Action::Character(character_action) => {
            // Use the new effect-based execution system
            execute::pre_validate(character_action, state, env).map_err(|error| {
                ExecuteError::Character(TransitionPhaseError::new(
                    TransitionPhase::PreValidate,
                    error,
                ))
            })?;

            let result = execute::apply(character_action, state, env).map_err(|error| {
                ExecuteError::Character(TransitionPhaseError::new(TransitionPhase::Apply, error))
            })?;

            execute::post_validate(character_action, state, env).map_err(|error| {
                ExecuteError::Character(TransitionPhaseError::new(
                    TransitionPhase::PostValidate,
                    error,
                ))
            })?;

            Ok(Some(result))
        }
        Action::System { kind } => {
            match kind {
                SystemActionKind::PrepareTurn(transition) => {
                    drive_transition(transition, state, env).map_err(ExecuteError::PrepareTurn)?;
                }
                SystemActionKind::ActionCost(transition) => {
                    drive_transition(transition, state, env).map_err(ExecuteError::ActionCost)?;
                }
                SystemActionKind::Activation(transition) => {
                    drive_transition(transition, state, env).map_err(ExecuteError::Activation)?;
                }
                SystemActionKind::RemoveFromActive(transition) => {
                    drive_transition(transition, state, env)
                        .map_err(ExecuteError::RemoveFromActive)?;
                }
            }
            Ok(None)
        }
    }
}
