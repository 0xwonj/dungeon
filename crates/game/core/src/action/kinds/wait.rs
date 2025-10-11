use std::convert::Infallible;

use crate::action::ActionTransition;
use crate::env::GameEnv;
use crate::state::{EntityId, GameState, Tick};

/// Wait action that does nothing but consumes time.
///
/// This is useful for NPCs that want to pause, or for players to let time pass.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WaitAction {
    pub actor: EntityId,
}

impl WaitAction {
    pub fn new(actor: EntityId) -> Self {
        Self { actor }
    }
}

impl ActionTransition for WaitAction {
    type Error = Infallible;

    fn cost(&self) -> Tick {
        Tick(100)
    }

    fn pre_validate(&self, _state: &GameState, _env: &GameEnv<'_>) -> Result<(), Self::Error> {
        // Wait action always succeeds
        Ok(())
    }

    fn apply(&self, _state: &mut GameState, _env: &GameEnv<'_>) -> Result<(), Self::Error> {
        // Wait action doesn't modify state
        Ok(())
    }

    fn post_validate(&self, _state: &GameState, _env: &GameEnv<'_>) -> Result<(), Self::Error> {
        // Nothing to validate after waiting
        Ok(())
    }
}
