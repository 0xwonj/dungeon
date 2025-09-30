use core::convert::Infallible;

use crate::action::ActionTransition;
use crate::state::{EntityId, GameState};

/// Performs an interaction with a nearby prop or entity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InteractAction {
    pub target: EntityId,
}

impl InteractAction {
    pub fn new(target: EntityId) -> Self {
        Self { target }
    }
}

impl<Env> ActionTransition<Env> for InteractAction {
    type Error = Infallible;

    fn apply(&self, _state: &mut GameState, _env: &Env) -> Result<(), Self::Error> {
        Ok(())
    }
}
