use crate::action::ActionTransition;
use crate::action::{AttackAction, InteractAction, MoveAction, UseItemAction};

/// Identifies which stage of the transition pipeline produced an error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransitionPhase {
    PreValidate,
    Apply,
    PostValidate,
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

/// Errors surfaced while executing an action through the game engine.
#[derive(Clone, Debug)]
pub enum ExecuteError {
    Move(TransitionPhaseError<<MoveAction as ActionTransition>::Error>),
    Attack(TransitionPhaseError<<AttackAction as ActionTransition>::Error>),
    UseItem(TransitionPhaseError<<UseItemAction as ActionTransition>::Error>),
    Interact(TransitionPhaseError<<InteractAction as ActionTransition>::Error>),
}
