//! Action transition dispatch and execution logic.

use crate::action::{Action, ActionTransition, CharacterActionKind, SystemActionKind};
use crate::env::GameEnv;
use crate::state::GameState;

use super::errors::{ExecuteError, TransitionPhase, TransitionPhaseError};

pub(super) type TransitionResult<E> = Result<(), TransitionPhaseError<E>>;

/// Dispatches an action to its appropriate transition handler.
///
/// This macro routes actions to their specific transition implementations
/// and wraps errors in the appropriate ExecuteError variant.
macro_rules! dispatch_transition {
    ($action:expr, $reducer:expr, $env:expr) => {{
        match $action {
            Action::Character { kind, .. } => match kind {
                CharacterActionKind::Move(transition) => {
                    drive_transition(transition, $reducer, $env).map_err(ExecuteError::Move)
                }
                CharacterActionKind::Attack(transition) => {
                    drive_transition(transition, $reducer, $env).map_err(ExecuteError::Attack)
                }
                CharacterActionKind::UseItem(transition) => {
                    drive_transition(transition, $reducer, $env).map_err(ExecuteError::UseItem)
                }
                CharacterActionKind::Interact(transition) => {
                    drive_transition(transition, $reducer, $env).map_err(ExecuteError::Interact)
                }
                CharacterActionKind::Wait => Ok(()),
            },
            Action::System { kind } => match kind {
                SystemActionKind::PrepareTurn(transition) => {
                    drive_transition(transition, $reducer, $env).map_err(ExecuteError::PrepareTurn)
                }
                SystemActionKind::ActionCost(transition) => {
                    drive_transition(transition, $reducer, $env).map_err(ExecuteError::ActionCost)
                }
                SystemActionKind::Activation(transition) => {
                    drive_transition(transition, $reducer, $env).map_err(ExecuteError::Activation)
                }
            },
        }
    }};
}

/// Executes a transition through the three-phase pipeline.
///
/// Phases:
/// 1. `pre_validate` - Check preconditions before mutation
/// 2. `apply` - Mutate the game state
/// 3. `post_validate` - Verify postconditions after mutation
#[inline]
fn drive_transition<T>(
    transition: &T,
    state: &mut GameState,
    env: &GameEnv<'_>,
) -> TransitionResult<T::Error>
where
    T: ActionTransition,
{
    transition
        .pre_validate(state, env)
        .map_err(|error| TransitionPhaseError::new(TransitionPhase::PreValidate, error))?;

    transition
        .apply(state, env)
        .map_err(|error| TransitionPhaseError::new(TransitionPhase::Apply, error))?;

    transition
        .post_validate(state, env)
        .map_err(|error| TransitionPhaseError::new(TransitionPhase::PostValidate, error))
}

/// Executes an action through the transition pipeline.
///
/// This is the internal implementation used by GameEngine::execute().
pub(super) fn execute_transition(
    action: &Action,
    state: &mut GameState,
    env: &GameEnv<'_>,
) -> Result<(), ExecuteError> {
    dispatch_transition!(action, state, env)
}
