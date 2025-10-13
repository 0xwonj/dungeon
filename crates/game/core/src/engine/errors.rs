use crate::action::ActionTransition;
use crate::action::{
    ActionCostAction, ActivationAction, AttackAction, InteractAction, MoveAction,
    PrepareTurnAction, UseItemAction,
};

/// Identifies which stage of the transition pipeline produced an error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
pub enum ExecuteError {
    #[error("move action failed: {0}")]
    Move(TransitionPhaseError<<MoveAction as ActionTransition>::Error>),

    #[error("attack action failed: {0}")]
    Attack(TransitionPhaseError<<AttackAction as ActionTransition>::Error>),

    #[error("use item action failed: {0}")]
    UseItem(TransitionPhaseError<<UseItemAction as ActionTransition>::Error>),

    #[error("interact action failed: {0}")]
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
