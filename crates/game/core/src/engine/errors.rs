//! Error types for action execution pipeline.

use crate::action::{
    ActionCostAction, ActionTransition, ActivationAction, AttackAction, InteractAction, MoveAction,
    PrepareTurnAction, UseItemAction, WaitAction,
};

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

    #[error("wait action failed: {0}")]
    #[cfg_attr(feature = "serde", serde(skip))]
    Wait(TransitionPhaseError<<WaitAction as ActionTransition>::Error>),

    #[error("prepare turn action failed: {0}")]
    PrepareTurn(TransitionPhaseError<<PrepareTurnAction as ActionTransition>::Error>),

    #[error("action cost action failed: {0}")]
    ActionCost(TransitionPhaseError<<ActionCostAction as ActionTransition>::Error>),

    #[error("activation action failed: {0}")]
    Activation(TransitionPhaseError<<ActivationAction as ActionTransition>::Error>),

    #[error("hook chain too deep: hook '{hook_name}' reached depth {depth}")]
    HookChainTooDeep { hook_name: String, depth: usize },

    #[error("invalid actor: system action must be executed by SYSTEM (got {actor})")]
    SystemActionNotFromSystem { actor: crate::state::EntityId },

    #[error(
        "invalid actor: action actor {actor} does not match current turn actor {current_actor}"
    )]
    ActorNotCurrent {
        actor: crate::state::EntityId,
        current_actor: crate::state::EntityId,
    },
}
