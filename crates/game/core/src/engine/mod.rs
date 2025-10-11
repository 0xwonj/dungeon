//! Turn scheduling and action execution pipeline.
//!
//! The [`GameEngine`] is the authoritative reducer for [`GameState`]. It
//! orchestrates the transition phases, applies costs, and surfaces rich error
//! information for the runtime.
mod errors;
mod hook;
mod turns;

use std::sync::Arc;

use crate::action::{Action, ActionKind, ActionTransition};
use crate::env::GameEnv;
use crate::state::{GameState, StateDelta};

pub use errors::{ExecuteError, TransitionPhase, TransitionPhaseError};
pub use hook::{ActionCostHook, ActivationHook, PostExecutionHook};
pub use turns::TurnError;

type TransitionResult<E> = Result<(), TransitionPhaseError<E>>;

macro_rules! dispatch_transition {
    ($action:expr, $reducer:expr, $env:expr, { $($variant:ident => $err:ident),+ $(,)? }) => {{
        match &$action.kind {
            $(
                ActionKind::$variant(transition) => {
                    drive_transition(transition, $reducer, $env).map_err(ExecuteError::$err)
                }
            )+
        }
    }};
}

/// Game engine that manages action execution, turn scheduling, and game logic.
///
/// Combines action execution with turn scheduling in a unified API.
/// Turn scheduling uses simple linear search over active actors for simplicity and correctness.
pub struct GameEngine<'a> {
    state: &'a mut GameState,
    hooks: Arc<[Arc<dyn PostExecutionHook>]>,
}

impl<'a> GameEngine<'a> {
    /// Creates a new game engine with the given state and configuration.
    pub fn new(state: &'a mut GameState) -> Self {
        Self {
            state,
            hooks: hook::default_hooks(),
        }
    }

    /// Executes an action by routing it through the appropriate transition pipeline.
    /// After successful execution, applies post-execution hooks and returns the resulting [`StateDelta`].
    pub fn execute(
        &mut self,
        env: GameEnv<'_>,
        action: &Action,
    ) -> Result<StateDelta, ExecuteError> {
        let before = self.state.clone();

        dispatch_transition!(action, self.state, &env, {
            Move => Move,
            Attack => Attack,
            UseItem => UseItem,
            Interact => Interact,
            Wait => Wait,
        })?;

        // Generate initial delta to check what changed
        let initial_delta = StateDelta::from_states(action.clone(), &before, self.state);

        // Apply post-execution hooks (already sorted by priority)
        for hook in self.hooks.iter() {
            if hook.should_trigger(&initial_delta) {
                hook.apply(self.state, &initial_delta, &env);
            }
        }

        // Generate final delta that includes hook effects
        let final_delta = StateDelta::from_states(action.clone(), &before, self.state);
        Ok(final_delta)
    }
}

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
