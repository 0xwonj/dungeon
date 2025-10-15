//! Turn scheduling and action execution pipeline.
//!
//! The [`GameEngine`] is the authoritative reducer for [`GameState`]. It
//! orchestrates the transition phases and surfaces rich error information
//! for the runtime. All state mutations, including system actions for turn
//! scheduling and cost application, flow through the same execute() pipeline.

use crate::action::{Action, ActionKind, ActionTransition};
use crate::action::{
    ActionCostAction, ActivationAction, AttackAction, InteractAction, MoveAction,
    PrepareTurnAction, UseItemAction,
};
use crate::env::GameEnv;
use crate::state::{GameState, StateDelta};

/// Identifies which stage of the transition pipeline produced an error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TransitionPhase {
    PreValidate,
    Apply,
    PostValidate,
}

impl TransitionPhase {
    pub fn as_str(&self) -> &'static str {
        match self {
            TransitionPhase::PreValidate => "pre_validate",
            TransitionPhase::Apply => "apply",
            TransitionPhase::PostValidate => "post_validate",
        }
    }
}

/// Associates a transition phase with the underlying error.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TransitionPhaseError<E> {
    pub phase: TransitionPhase,
    pub error: E,
}

impl<E> TransitionPhaseError<E> {
    pub fn new(phase: TransitionPhase, error: E) -> Self {
        Self { phase, error }
    }
}

impl<E: std::fmt::Display> std::fmt::Display for TransitionPhaseError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} failed: {}", self.phase.as_str(), self.error)
    }
}

impl<E: std::fmt::Display + std::fmt::Debug> std::error::Error for TransitionPhaseError<E> {}

/// Errors surfaced while executing an action through the game engine.
#[derive(Clone, Debug, thiserror::Error)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ExecuteError {
    #[error("move action failed: {0}")]
    Move(TransitionPhaseError<<MoveAction as ActionTransition>::Error>),

    #[error("attack action failed: {0}")]
    #[cfg_attr(feature = "serde", serde(skip))]
    Attack(TransitionPhaseError<<AttackAction as ActionTransition>::Error>),

    #[error("use item action failed: {0}")]
    #[cfg_attr(feature = "serde", serde(skip))]
    UseItem(TransitionPhaseError<<UseItemAction as ActionTransition>::Error>),

    #[error("interact action failed: {0}")]
    #[cfg_attr(feature = "serde", serde(skip))]
    Interact(TransitionPhaseError<<InteractAction as ActionTransition>::Error>),

    #[error("prepare turn action failed: {0}")]
    PrepareTurn(TransitionPhaseError<<PrepareTurnAction as ActionTransition>::Error>),

    #[error("action cost action failed: {0}")]
    ActionCost(TransitionPhaseError<<ActionCostAction as ActionTransition>::Error>),

    #[error("activation action failed: {0}")]
    Activation(TransitionPhaseError<<ActivationAction as ActionTransition>::Error>),

    #[error("hook chain too deep: hook '{hook_name}' reached depth {depth}")]
    HookChainTooDeep { hook_name: String, depth: usize },
}

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
