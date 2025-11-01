use crate::action::ActionTransition;
use crate::env::GameEnv;
use crate::error::NeverError;
use crate::state::{EntityId, GameState, Tick};

/// Wait action - actor passes their turn without performing any action.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct WaitAction {
    pub actor: EntityId,
}

impl WaitAction {
    pub fn new(actor: EntityId) -> Self {
        Self { actor }
    }
}

impl ActionTransition for WaitAction {
    type Error = NeverError;
    type Result = ();

    fn actor(&self) -> EntityId {
        self.actor
    }

    fn cost(&self, env: &GameEnv<'_>) -> Tick {
        env.tables().map(|t| t.action_costs().wait).unwrap_or(100)
    }

    fn apply(&self, _state: &mut GameState, _env: &GameEnv<'_>) -> Result<(), Self::Error> {
        Ok(())
    }
}
