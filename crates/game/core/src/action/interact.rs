use core::convert::Infallible;

use crate::action::ActionTransition;
use crate::env::GameEnv;
use crate::state::{EntityId, GameState, Tick};

/// Performs an interaction with a nearby prop or entity.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct InteractAction {
    pub actor: EntityId,
    pub target: EntityId,
}

impl InteractAction {
    pub fn new(actor: EntityId, target: EntityId) -> Self {
        Self { actor, target }
    }
}

impl ActionTransition for InteractAction {
    type Error = Infallible;

    fn actor(&self) -> EntityId {
        self.actor
    }

    fn cost(&self) -> Tick {
        5
    }

    fn apply(&self, _state: &mut GameState, _env: &GameEnv<'_>) -> Result<(), Self::Error> {
        Ok(())
    }
}
