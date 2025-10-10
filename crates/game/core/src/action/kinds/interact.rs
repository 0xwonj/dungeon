use core::convert::Infallible;

use crate::action::ActionTransition;
use crate::engine::StateReducer;
use crate::env::GameEnv;
use crate::state::EntityId;

/// Performs an interaction with a nearby prop or entity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InteractAction {
    pub actor: EntityId,
    pub target: EntityId,
}

impl InteractAction {
    pub fn new(actor: EntityId, target: EntityId) -> Self {
        Self { actor, target }
    }
}

/// Command describing a generic interaction intent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InteractCommand {
    pub target: EntityId,
}

impl InteractCommand {
    pub fn new(target: EntityId) -> Self {
        Self { target }
    }
}

impl ActionTransition for InteractAction {
    type Error = Infallible;

    fn cost(&self) -> crate::state::Tick {
        crate::state::Tick(5)
    }

    fn apply(
        &self,
        _reducer: &mut StateReducer<'_>,
        _env: &GameEnv<'_>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
}
