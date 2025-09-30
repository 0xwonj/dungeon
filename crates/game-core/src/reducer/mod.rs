use crate::action::{
    Action, ActionKind, ActionTransition, AttackAction, InteractAction, MoveAction, UseItemAction,
};
use crate::state::GameState;

type TransitionResult<E> = Result<(), TransitionPhaseError<E>>;

macro_rules! dispatch_transition {
    ($action:expr, $state:expr, $env:expr, { $($variant:ident => $err:ident),+ $(,)? }) => {{
        match &$action.kind {
            $(
                ActionKind::$variant(transition) => {
                    drive_transition(transition, $state, $env).map_err(StepError::$err)
                }
            )+
            ActionKind::Wait => Ok(()),
        }
    }};
}

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

/// Errors surfaced while executing an action through the reducer.
#[derive(Clone, Debug)]
pub enum StepError<Env> {
    Move(TransitionPhaseError<<MoveAction as ActionTransition<Env>>::Error>),
    Attack(TransitionPhaseError<<AttackAction as ActionTransition<Env>>::Error>),
    UseItem(TransitionPhaseError<<UseItemAction as ActionTransition<Env>>::Error>),
    Interact(TransitionPhaseError<<InteractAction as ActionTransition<Env>>::Error>),
}

/// Drives the state machine by routing actions through their transition hooks.
pub fn step<Env>(state: &mut GameState, env: &Env, action: &Action) -> Result<(), StepError<Env>>
where
    MoveAction: ActionTransition<Env>,
    AttackAction: ActionTransition<Env>,
    UseItemAction: ActionTransition<Env>,
    InteractAction: ActionTransition<Env>,
{
    dispatch_transition!(action, state, env, {
        Move => Move,
        Attack => Attack,
        UseItem => UseItem,
        Interact => Interact,
    })
}

#[inline]
fn drive_transition<T, Env>(
    transition: &T,
    state: &mut GameState,
    env: &Env,
) -> TransitionResult<T::Error>
where
    T: ActionTransition<Env>,
{
    transition
        .pre_validate(&*state, env)
        .map_err(|error| TransitionPhaseError::new(TransitionPhase::PreValidate, error))?;

    transition
        .apply(state, env)
        .map_err(|error| TransitionPhaseError::new(TransitionPhase::Apply, error))?;

    transition
        .post_validate(&*state, env)
        .map_err(|error| TransitionPhaseError::new(TransitionPhase::PostValidate, error))
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::{Action, ActionKind, CardinalDirection, MoveAction};
    use crate::state::{EntityId, GameState};

    #[derive(Debug, Default)]
    struct StubEnv;

    #[test]
    fn step_routes_to_transition_hooks() {
        let mut state = GameState::default();
        let env = StubEnv::default();
        let action = Action::new(
            EntityId::PLAYER,
            ActionKind::Move(MoveAction::new(CardinalDirection::North)),
        );

        step(&mut state, &env, &action).expect("stub transition should succeed");
    }
}
