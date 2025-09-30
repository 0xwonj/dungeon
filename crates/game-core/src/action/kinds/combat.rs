use core::convert::Infallible;

use crate::action::ActionTransition;
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

impl<Env> ActionTransition<Env> for AttackAction {
    type Error = Infallible;

    fn apply(&self, _state: &mut GameState, _env: &Env) -> Result<(), Self::Error> {
        Ok(())
    }
}
