use core::convert::Infallible;

use crate::action::ActionTransition;
use crate::env::GameEnv;
use crate::state::{EntityId, GameState};

/// Offensive action against a target entity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttackAction {
    pub target: EntityId,
    pub style: AttackStyle,
}

impl AttackAction {
    pub fn new(target: EntityId, style: AttackStyle) -> Self {
        Self { target, style }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AttackStyle {
    Melee,
}

impl ActionTransition for AttackAction {
    type Error = Infallible;

    fn apply(&self, _state: &mut GameState, _env: &GameEnv<'_>) -> Result<(), Self::Error> {
        Ok(())
    }
}
