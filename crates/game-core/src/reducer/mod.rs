use crate::action::{
    Action, ActionKind, ActionTransition, AttackAction, InteractAction, MoveAction, UseItemAction,
};
use crate::env::GameEnv;
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
pub enum StepError {
    Move(TransitionPhaseError<<MoveAction as ActionTransition>::Error>),
    Attack(TransitionPhaseError<<AttackAction as ActionTransition>::Error>),
    UseItem(TransitionPhaseError<<UseItemAction as ActionTransition>::Error>),
    Interact(TransitionPhaseError<<InteractAction as ActionTransition>::Error>),
}

/// Drives the state machine by routing actions through their transition hooks.
pub fn step(state: &mut GameState, env: GameEnv<'_>, action: &Action) -> Result<(), StepError> {
    dispatch_transition!(action, state, env, {
        Move => Move,
        Attack => Attack,
        UseItem => UseItem,
        Interact => Interact,
    })
}

#[inline]
fn drive_transition<T>(
    transition: &T,
    state: &mut GameState,
    env: GameEnv<'_>,
) -> TransitionResult<T::Error>
where
    T: ActionTransition,
{
    transition
        .pre_validate(&*state, &env)
        .map_err(|error| TransitionPhaseError::new(TransitionPhase::PreValidate, error))?;

    transition
        .apply(state, &env)
        .map_err(|error| TransitionPhaseError::new(TransitionPhase::Apply, error))?;

    transition
        .post_validate(&*state, &env)
        .map_err(|error| TransitionPhaseError::new(TransitionPhase::PostValidate, error))
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::{Action, ActionKind, CardinalDirection, MoveAction};
    use crate::env::{
        AttackProfile, Env, ItemCategory, ItemDefinition, ItemOracle, MapDimensions, MapOracle,
        MovementRules, StaticTile, TablesOracle, TerrainKind,
    };
    use crate::state::{EntityId, GameState, ItemHandle, Position};

    #[derive(Debug, Default)]
    struct StubMap;

    impl MapOracle for StubMap {
        fn dimensions(&self) -> MapDimensions {
            MapDimensions::new(4, 4)
        }

        fn tile(&self, position: Position) -> Option<StaticTile> {
            if self.dimensions().contains(position) {
                Some(StaticTile::new(TerrainKind::Floor))
            } else {
                None
            }
        }
    }

    #[derive(Debug, Default)]
    struct StubItems;

    impl ItemOracle for StubItems {
        fn definition(&self, handle: ItemHandle) -> Option<ItemDefinition> {
            Some(ItemDefinition::new(handle, ItemCategory::Utility, None, None))
        }
    }

    #[derive(Debug, Default)]
    struct StubTables;

    impl TablesOracle for StubTables {
        fn movement_rules(&self) -> MovementRules {
            MovementRules::new(1, 1)
        }

        fn attack_profile(&self, _style: crate::action::AttackStyle) -> Option<AttackProfile> {
            Some(AttackProfile::new(1, 0))
        }
    }

    #[test]
    fn step_routes_to_transition_hooks() {
        let mut state = GameState::default();
        static MAP: StubMap = StubMap;
        static ITEMS: StubItems = StubItems;
        static TABLES: StubTables = StubTables;
        let env = Env::with_all(&MAP, &ITEMS, &TABLES).into_game_env();
        let action = Action::new(
            EntityId::PLAYER,
            ActionKind::Move(MoveAction::new(CardinalDirection::North)),
        );

        step(&mut state, env, &action).expect("stub transition should succeed");
    }
}
