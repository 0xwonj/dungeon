//! Action transition dispatch and execution logic.

use crate::action::{Action, ActionTransition, CharacterActionKind, SystemActionKind};
use crate::env::GameEnv;
use crate::state::GameState;

use super::ActionResult;
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

/// Executes an action through the transition pipeline and returns ActionResult.
///
/// This is the internal implementation used by GameEngine::execute().
/// Routes each action type to its transition and wraps the result in ActionResult.
pub(super) fn execute_transition(
    action: &Action,
    state: &mut GameState,
    env: &GameEnv<'_>,
) -> Result<ActionResult, ExecuteError> {
    match action {
        Action::Character { kind, .. } => match kind {
            CharacterActionKind::Move(transition) => {
                drive_transition(transition, state, env).map_err(ExecuteError::Move)?;
                Ok(ActionResult::Move)
            }
            CharacterActionKind::Attack(transition) => {
                let attack_result =
                    drive_transition(transition, state, env).map_err(ExecuteError::Attack)?;
                Ok(ActionResult::Attack(attack_result))
            }
            CharacterActionKind::UseItem(transition) => {
                drive_transition(transition, state, env).map_err(ExecuteError::UseItem)?;
                Ok(ActionResult::UseItem)
            }
            CharacterActionKind::Interact(transition) => {
                drive_transition(transition, state, env).map_err(ExecuteError::Interact)?;
                Ok(ActionResult::Interact)
            }
            CharacterActionKind::Wait(transition) => {
                drive_transition(transition, state, env).map_err(ExecuteError::Wait)?;
                Ok(ActionResult::Wait)
            }
        },
        Action::System { kind } => match kind {
            SystemActionKind::PrepareTurn(transition) => {
                drive_transition(transition, state, env).map_err(ExecuteError::PrepareTurn)?;
                Ok(ActionResult::System)
            }
            SystemActionKind::ActionCost(transition) => {
                drive_transition(transition, state, env).map_err(ExecuteError::ActionCost)?;
                Ok(ActionResult::System)
            }
            SystemActionKind::Activation(transition) => {
                drive_transition(transition, state, env).map_err(ExecuteError::Activation)?;
                Ok(ActionResult::System)
            }
        },
    }
}
