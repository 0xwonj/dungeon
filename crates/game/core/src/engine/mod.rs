//! Turn scheduling and action execution pipeline.
//!
//! The [`GameEngine`] is the authoritative reducer for [`GameState`]. It
//! orchestrates the transition phases and surfaces rich error information
//! for the runtime. All state mutations, including system actions for turn
//! scheduling and cost application, flow through the same execute() pipeline.
mod errors;
mod hook;
mod turns;

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
            ActionKind::Wait => Ok(()),
        }
    }};
}

/// Game engine that manages action execution, turn scheduling, and game logic.
///
/// All state mutations flow through the three-phase action pipeline:
/// pre_validate → apply → post_validate
///
/// Both player/NPC actions and system actions (turn scheduling, cost application,
/// entity activation) use the same execution path, ensuring complete auditability
/// and proof generation for all state changes.
pub struct GameEngine<'a> {
    state: &'a mut GameState,
}

impl<'a> GameEngine<'a> {
    /// Creates a new game engine with the given state.
    pub fn new(state: &'a mut GameState) -> Self {
        Self { state }
    }

    /// Executes an action by routing it through the appropriate transition pipeline.
    ///
    /// Returns a [`StateDelta`] capturing all state changes made by the action.
    /// Both player/NPC actions and system actions go through the same pipeline.
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
            PrepareTurn => PrepareTurn,
            ActionCost => ActionCost,
            Activation => Activation,
        })?;

        // Generate delta capturing all state changes
        let delta = StateDelta::from_states(action.clone(), &before, self.state);
        Ok(delta)
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
